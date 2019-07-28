use std::cmp::Ordering;
use std::collections::vec_deque::VecDeque;
use crate::vm::chunk::CompiledModule;
use crate::vm::opcode::Opcode;
use crate::vm::value::{Value, Obj};
use crate::vm::compiler::MAIN_CHUNK_NAME;
use std::collections::HashMap;
use std::cell::Cell;

#[derive(Debug)]
pub enum InterpretError {
    StackEmpty,
    ConstIdxOutOfBounds,
    EndOfBytes,
}

struct CallFrame {
    ip: usize,
    chunk_name: String,
    stack_offset: usize,
}

pub struct VM<'a> {
    call_stack: Vec<CallFrame>,
    module: &'a mut CompiledModule<'a>,
    stack: Vec<Value>,
    globals: HashMap<String, Value>,
}

impl<'a> VM<'a> {
    pub fn new(module: &'a mut CompiledModule<'a>) -> Self {
        let root_frame = CallFrame {
            ip: 0,
            chunk_name: MAIN_CHUNK_NAME.to_owned(),
            stack_offset: 0,
        };

        VM {
            call_stack: vec![root_frame],
            module,
            stack: Vec::new(),
            globals: HashMap::new(),
        }
    }

    fn stack_insert_at(&mut self, index: usize, value: Value) {
        match self.stack.get_mut(index) {
            Some(slot) => *slot = value,
            None => panic!("No stack slot available at index {}", index)
        }
    }

    fn stack_get(&mut self, index: usize) -> Value {
        self.stack.get(index)
            .map(|value| value.clone())
            .unwrap_or(Value::Nil)
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Option<Value> {
        self.stack.pop()
    }

    fn pop_expect(&mut self) -> Result<Value, InterpretError> {
        self.stack.pop().ok_or(InterpretError::StackEmpty)
    }

    fn curr_frame(&self) -> &CallFrame {
        self.call_stack.last().expect("There needs to be at least 1 active call stack member")
    }

    fn read_byte(&mut self) -> Option<u8> {
        let CallFrame { chunk_name: curr_chunk_name, .. } = self.curr_frame();
        let chunk = self.module.get_chunk(curr_chunk_name.to_string()).unwrap();

        let frame = self.call_stack.last_mut().unwrap();
        if chunk.code.len() == frame.ip {
            None
        } else {
            let instr = chunk.code[frame.ip];
            frame.ip += 1;
            Some(instr)
        }
    }

    fn read_byte_expect(&mut self) -> Result<usize, InterpretError> {
        self.read_byte()
            .map(|b| b as usize)
            .ok_or(InterpretError::EndOfBytes)
    }

    fn read_instr(&mut self) -> Option<Opcode> {
        self.read_byte().map(|b| Opcode::from(b))
    }

    fn int_op<F>(&mut self, f: F) -> Result<(), InterpretError>
        where F: FnOnce(i64, i64) -> i64
    {
        let b = self.pop_expect()?;
        let a = self.pop_expect()?;

        match (a, b) {
            (Value::Int(a), Value::Int(b)) => {
                self.push(Value::Int(f(a, b)))
            }
            _ => unreachable!()
        };
        Ok(())
    }

    fn float_op<F>(&mut self, f: F) -> Result<(), InterpretError>
        where F: FnOnce(f64, f64) -> f64
    {
        let b = self.pop_expect()?;
        let a = self.pop_expect()?;

        match (a, b) {
            (Value::Float(a), Value::Float(b)) => {
                self.push(Value::Float(f(a, b)))
            }
            _ => unreachable!()
        };
        Ok(())
    }

    fn comp_values(&mut self, opcode: Opcode) -> Result<(), InterpretError> {
        let b = self.pop_expect()?;
        let a = self.pop_expect()?;

        // Rust can't natively compare floats and ints, so we provide that logic here via
        // partial_cmp, deferring to normal implementation of PartialOrd for non-float comparison.
        let ord = match (a, b) {
            (Value::Int(a), Value::Float(b)) => (a as f64).partial_cmp(&b),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(&b),
            (Value::Float(a), Value::Int(b)) => a.partial_cmp(&(b as f64)),
            (a @ _, b @ _) => a.partial_cmp(&b),
        };

        // If the partial_cmp call above returns None, treat that as an equivalence value of false
        let res = match ord {
            None => false,
            Some(ord) =>
                match opcode {
                    Opcode::LT => ord == Ordering::Less,
                    Opcode::LTE => ord != Ordering::Greater,
                    Opcode::GT => ord == Ordering::Greater,
                    Opcode::GTE => ord != Ordering::Less,
                    Opcode::Eq => ord == Ordering::Equal,
                    Opcode::Neq => ord != Ordering::Equal,
                    _ => unreachable!()
                }
        };
        self.push(Value::Bool(res));

        Ok(())
    }

    fn store(&mut self, stack_slot: usize) -> Result<(), InterpretError> {
        let stack_slot = stack_slot + self.call_stack.last().unwrap().stack_offset;
        let value = self.pop_expect()?;
        Ok(self.stack_insert_at(stack_slot, value)) // TODO: Raise InterpretError when OOB stack_slot
    }

    fn load(&mut self, stack_slot: usize) -> Result<(), InterpretError> {
        let stack_slot = stack_slot + self.call_stack.last().unwrap().stack_offset;
        let value = self.stack_get(stack_slot);
        Ok(self.push(value))
    }

    pub fn run(&mut self) -> Result<Option<Value>, InterpretError> {
        loop {
            let instr = self.read_instr()
                .ok_or(InterpretError::EndOfBytes)?;

            match instr {
                Opcode::Constant => {
                    let const_idx = self.read_byte_expect()?;
                    let val = self.module.constants.get(const_idx)
                        .ok_or(InterpretError::ConstIdxOutOfBounds)?
                        .clone();
                    self.push(val)
                }
                Opcode::Nil => self.push(Value::Nil),
                Opcode::IConst0 => self.push(Value::Int(0)),
                Opcode::IConst1 => self.push(Value::Int(1)),
                Opcode::IConst2 => self.push(Value::Int(2)),
                Opcode::IConst3 => self.push(Value::Int(3)),
                Opcode::IConst4 => self.push(Value::Int(4)),
                Opcode::IAdd => self.int_op(|a, b| a + b)?,
                Opcode::ISub => self.int_op(|a, b| a - b)?,
                Opcode::IMul => self.int_op(|a, b| a * b)?,
                Opcode::IDiv => self.int_op(|a, b| a / b)?,
                Opcode::FAdd => self.float_op(|a, b| a + b)?,
                Opcode::FSub => self.float_op(|a, b| a - b)?,
                Opcode::FMul => self.float_op(|a, b| a * b)?,
                Opcode::FDiv => self.float_op(|a, b| a / b)?,
                Opcode::I2F => {
                    let val = self.pop_expect()?;
                    let val = match val {
                        Value::Int(v) => Value::Float(v as f64),
                        _ => unreachable!()
                    };
                    self.push(val)
                }
                Opcode::F2I => {
                    let val = self.pop_expect()?;
                    let val = match val {
                        Value::Float(v) => Value::Int(v as i64),
                        _ => unreachable!()
                    };
                    self.push(val)
                }
                Opcode::Invert => {
                    let val = self.pop_expect()?;
                    let val = match val {
                        Value::Int(v) => Value::Int(-v),
                        Value::Float(v) => Value::Float(-v),
                        _ => unreachable!()
                    };
                    self.push(val)
                }
                Opcode::StrConcat => {
                    let b = self.pop_expect()?;
                    let a = self.pop_expect()?;

                    let a = a.to_string();
                    let b = b.to_string();
                    let concat = a + &b;
                    self.push(Value::Obj(Obj::StringObj { value: Box::new(concat) }))
                }
                Opcode::T => self.push(Value::Bool(true)),
                Opcode::F => self.push(Value::Bool(false)),
                Opcode::And | Opcode::Or => {
                    // TODO: Short-circuiting
                    if let Value::Bool(b) = self.pop_expect()? {
                        if let Value::Bool(a) = self.pop_expect()? {
                            let res = if let Opcode::And = instr {
                                a && b
                            } else {
                                a || b
                            };
                            self.push(Value::Bool(res));
                        } else {
                            unreachable!()
                        }
                    } else {
                        unreachable!()
                    }
                }
                Opcode::Negate => {
                    if let Value::Bool(val) = self.pop_expect()? {
                        self.push(Value::Bool(!val));
                    } else {
                        unreachable!()
                    }
                }
                Opcode::Coalesce => { // TODO: Rewrite this using jumps when they're implemented!
                    let fallback = self.pop_expect()?;

                    if let Value::Obj(Obj::OptionObj { value }) = self.pop_expect()? {
                        match value {
                            Some(value) => self.push(*value),
                            None => self.push(fallback)
                        }
                    } else {
                        unreachable!()
                    }
                }
                Opcode::LT => self.comp_values(Opcode::LT)?,
                Opcode::LTE => self.comp_values(Opcode::LTE)?,
                Opcode::GT => self.comp_values(Opcode::GT)?,
                Opcode::GTE => self.comp_values(Opcode::GTE)?,
                Opcode::Neq => self.comp_values(Opcode::Neq)?,
                Opcode::Eq => self.comp_values(Opcode::Eq)?,
                Opcode::ArrMk => {
                    if let Value::Int(mut size) = self.pop_expect()? {
                        // Array items are on the stack in reverse order, pop them off in reverse
                        let mut arr_items = VecDeque::<Box<Value>>::with_capacity(size as usize);
                        while size > 0 {
                            size -= 1;
                            arr_items.push_front(Box::new(self.pop_expect()?));
                        }
                        self.push(Value::Obj(Obj::ArrayObj { value: arr_items.into() }));
                    } else {
                        unreachable!()
                    }
                }
                Opcode::ArrLoad => {
                    if let Value::Int(idx) = self.pop_expect()? {
                        let value = match self.pop_expect()? {
                            Value::Obj(Obj::StringObj { value }) => {
                                let len = value.len() as i64;
                                let idx = if idx < 0 { idx + len } else { idx };

                                let value = match (*value).chars().nth(idx as usize) {
                                    Some(ch) => Some(
                                        Box::new(
                                            Value::Obj(Obj::StringObj {
                                                value: Box::new(ch.to_string())
                                            })
                                        )
                                    ),
                                    None => None
                                };
                                Value::Obj(Obj::OptionObj { value })
                            }
                            Value::Obj(Obj::ArrayObj { value }) => {
                                let len = value.len() as i64;
                                let value = if idx < -len || idx >= len {
                                    None
                                } else {
                                    let idx = if idx < 0 { idx + len } else { idx };
                                    Some(value[idx as usize].clone())
                                };
                                Value::Obj(Obj::OptionObj { value })
                            }
                            _ => unreachable!()
                        };
                        self.push(value);
                    } else {
                        unreachable!()
                    }
                }
                Opcode::ArrSlc => {
                    #[inline]
                    fn get_range_endpoints(len: usize, start: i64, end: Value) -> (usize, usize) {
                        let len = len as i64;
                        let start = if start < 0 { start + len } else { start };
                        let end = match end {
                            Value::Int(end) => end,
                            Value::Nil => len,
                            _ => unreachable!()
                        };
                        let end = if end < 0 { end + len } else { end };
                        (start as usize, end as usize - start as usize)
                    }

                    let end = self.pop_expect()?;
                    let start = match self.pop_expect()? {
                        Value::Int(start) => start,
                        _ => unreachable!()
                    };

                    let value = match self.pop_expect()? {
                        Value::Obj(Obj::StringObj { value }) => {
                            let (start, len) = get_range_endpoints(value.len(), start, end);
                            let value = (*value).chars().skip(start).take(len).collect::<String>();
                            Value::Obj(Obj::StringObj { value: Box::new(value) })
                        }
                        Value::Obj(Obj::ArrayObj { value }) => {
                            let (start, len) = get_range_endpoints(value.len(), start, end);
                            let value = value.into_iter().skip(start).take(len).collect::<Vec<_>>();
                            Value::Obj(Obj::ArrayObj { value })
                        }
                        _ => unreachable!()
                    };
                    self.push(value);
                }
                Opcode::GStore => {
                    let global_name = if let Value::Obj(Obj::StringObj { value }) = self.pop_expect()? {
                        *value
                    } else {
                        unreachable!()
                    };
                    let value = self.pop_expect()?;
                    self.globals.insert(global_name, value);
                }
                Opcode::LStore0 => self.store(0)?,
                Opcode::LStore1 => self.store(1)?,
                Opcode::LStore2 => self.store(2)?,
                Opcode::LStore3 => self.store(3)?,
                Opcode::LStore4 => self.store(4)?,
                Opcode::LStore => {
                    let stack_slot = self.read_byte_expect()?;
                    self.store(stack_slot)?
                }
                Opcode::GLoad => {
                    let global_name = if let Value::Obj(Obj::StringObj { value }) = self.pop_expect()? {
                        *value
                    } else {
                        unreachable!()
                    };
                    let value = self.globals.get(&global_name)
                        .unwrap_or(&Value::Nil)
                        .clone();
                    self.push(value);
                }
                Opcode::LLoad0 => self.load(0)?,
                Opcode::LLoad1 => self.load(1)?,
                Opcode::LLoad2 => self.load(2)?,
                Opcode::LLoad3 => self.load(3)?,
                Opcode::LLoad4 => self.load(4)?,
                Opcode::LLoad => {
                    let stack_slot = self.read_byte_expect()?;
                    self.load(stack_slot)?
                }
                Opcode::Jump => {
                    let jump_offset = self.read_byte_expect()?;

                    let frame = self.call_stack.last_mut().unwrap();
                    frame.ip += jump_offset;
                }
                Opcode::JumpIfF => {
                    let jump_offset = self.read_byte_expect()?;
                    if let Value::Bool(cond) = self.pop_expect()? {
                        if !cond {
                            let frame = self.call_stack.last_mut().unwrap();
                            frame.ip += jump_offset;
                        }
                    } else {
                        unreachable!()
                    }
                }
                Opcode::Invoke => {
                    let func_name = match self.pop_expect()? {
                        Value::Obj(Obj::StringObj { value }) => *value,
                        _ => unreachable!()
                    };

                    let arity = self.read_byte_expect()?;

                    let frame = CallFrame {
                        ip: 0,
                        chunk_name: func_name,
                        stack_offset: self.stack.len() - arity,
                    };
                    self.call_stack.push(frame);
                }
                Opcode::Pop => {
                    self.pop_expect()?;
                }
                Opcode::Return => {
                    let CallFrame { chunk_name, .. } = self.curr_frame();

                    if chunk_name == MAIN_CHUNK_NAME {
                        let top = self.pop();
                        break Ok(top);
                    } else {
                        // Pop off current frame, so the next loop will resume with the previous frame
                        self.call_stack.pop();
                    }
                }
            }
        }
    }
}