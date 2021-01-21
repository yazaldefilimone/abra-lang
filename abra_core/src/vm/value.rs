use itertools::Itertools;
use std::hash::{Hash, Hasher};
use crate::builtins::native::Array;
use crate::builtins::native_value_trait::NativeValue;
use crate::builtins::native_fns::NativeFn;
use crate::common::util::integer_decode;
use crate::vm::vm;
use crate::vm::compiler::Upvalue;
use std::fmt::{Display, Formatter, Error};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::cell::RefCell;
use std::sync::Arc;

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd)]
pub struct FnValue {
    pub name: String,
    pub code: Vec<u8>,
    pub upvalues: Vec<Upvalue>,
    pub receiver: Option<Arc<RefCell<Obj>>>,
    pub has_return: bool,
}

impl Hash for FnValue {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.name.hash(hasher);
        self.code.hash(hasher);
        self.upvalues.hash(hasher);
        if let Some(receiver) = &self.receiver {
            (&*receiver.borrow()).hash(hasher);
        }
        self.has_return.hash(hasher);
        hasher.finish();
    }
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd)]
pub struct ClosureValue {
    pub name: String,
    pub code: Vec<u8>,
    pub captures: Vec<Arc<RefCell<vm::Upvalue>>>,
    pub receiver: Option<Arc<RefCell<Obj>>>,
    pub has_return: bool,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, PartialOrd)]
pub struct TypeValue {
    pub name: String,
    pub constructor: Option<fn(Vec<Value>) -> Value>,
    pub fields: Vec<String>,
    pub methods: Vec<(String, Value)>,
    pub static_fields: Vec<(String, Value)>,
}

impl TypeValue {
    pub fn construct(self, args: Vec<Value>) -> Value {
        if let Some(constructor) = self.constructor {
            constructor(args)
        } else {
            Value::new_instance_obj(Value::Type(self), args)
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, PartialOrd)]
pub struct EnumValue {
    pub name: String,
    pub variants: Vec<(String, EnumVariantObj)>,
    pub methods: Vec<(String, Value)>,
    pub static_fields: Vec<(String, Value)>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    /// Represents a compile-time string constant (ie. the name of a function, or the key of a map).
    /// These are only transient values and should not remain on the stack. Compare to an actual,
    /// heap-allocated, run-time Value::Obj(Obj::StringObj) value.
    Str(String),
    Obj(Arc<RefCell<Obj>>),
    Fn(FnValue),
    Closure(ClosureValue),
    NativeFn(NativeFn),
    Type(TypeValue),
    Enum(EnumValue),
    Nil,
}

impl Value {
    pub fn new_string_obj(value: String) -> Value {
        let str = Obj::StringObj(value);
        Value::Obj(Arc::new(RefCell::new(str)))
    }

    pub fn new_array_obj(values: Vec<Value>) -> Value {
        Array::new(values).init()
    }

    pub fn new_set_obj(values: HashSet<Value>) -> Value {
        let arr = Obj::SetObj(values);
        Value::Obj(Arc::new(RefCell::new(arr)))
    }

    pub fn new_tuple_obj(values: Vec<Value>) -> Value {
        let arr = Obj::TupleObj(values);
        Value::Obj(Arc::new(RefCell::new(arr)))
    }

    pub fn new_map_obj(items: HashMap<Value, Value>) -> Value {
        let map = Obj::MapObj(items);
        Value::Obj(Arc::new(RefCell::new(map)))
    }

    pub fn new_instance_obj(typ: Value, fields: Vec<Value>) -> Value {
        let inst = Obj::InstanceObj(InstanceObj { typ: Box::new(typ), fields, methods: vec![] });
        Value::Obj(Arc::new(RefCell::new(inst)))
    }

    pub fn new_native_instance_obj(typ: TypeValue, inst: Box<dyn NativeValue>) -> Value {
        let inst = Obj::NativeInstanceObj(NativeInstanceObj { typ, inst, methods: vec![] });
        Value::Obj(Arc::new(RefCell::new(inst)))
    }

    pub fn new_enum_variant_obj(evv: EnumVariantObj) -> Value {
        let inst = Obj::EnumVariantObj(evv);
        Value::Obj(Arc::new(RefCell::new(inst)))
    }

    pub fn as_int(&self) -> &i64 {
        if let Value::Int(i) = self { i } else { unreachable!() }
    }

    pub fn as_float(&self) -> &f64 {
        if let Value::Float(f) = self { f } else { unreachable!() }
    }

    pub fn as_bool(&self) -> &bool {
        if let Value::Bool(b) = self { b } else { unreachable!() }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Value::Int(v) => write!(f, "{}", v),
            Value::Float(v) => write!(f, "{}", v),
            Value::Bool(v) => write!(f, "{}", v),
            Value::Str(val) => write!(f, "{}", val),
            Value::Obj(o) => write!(f, "{}", &*o.borrow()),
            Value::Fn(FnValue { name, .. }) |
            Value::Closure(ClosureValue { name, .. }) => write!(f, "<func {}>", name),
            Value::NativeFn(NativeFn { name, .. }) => write!(f, "<func {}>", name),
            Value::Type(TypeValue { name, .. }) => write!(f, "<type {}>", name),
            Value::Enum(EnumValue { name, .. }) => write!(f, "<enum {}>", name),
            Value::Nil => write!(f, "None"),
        }
    }
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        match self {
            Value::Int(i) => i.hash(hasher),
            Value::Float(f) => integer_decode(*f).hash(hasher),
            Value::Bool(b) => b.hash(hasher),
            Value::Str(s) => s.hash(hasher),
            Value::Obj(o) => (&*o.borrow()).hash(hasher),
            Value::Fn(FnValue { name, code, upvalues, receiver, has_return }) => {
                name.hash(hasher);
                code.hash(hasher);
                upvalues.hash(hasher);
                if let Some(obj) = receiver {
                    let obj = &*obj.borrow();
                    obj.hash(hasher);
                }
                has_return.hash(hasher);
            }
            Value::Closure(ClosureValue { name, code, captures, receiver, has_return }) => {
                name.hash(hasher);
                code.hash(hasher);
                for capture in captures {
                    let uv = &*capture.borrow();
                    uv.hash(hasher);
                }
                if let Some(obj) = receiver {
                    let obj = &*obj.borrow();
                    obj.hash(hasher);
                }
                has_return.hash(hasher);
            }
            Value::NativeFn(NativeFn { name, receiver, has_return, .. }) => {
                name.hash(hasher);
                if let Some(receiver) = receiver {
                    receiver.hash(hasher);
                }
                has_return.hash(hasher);
            }
            Value::Type(tv) => tv.hash(hasher),
            Value::Enum(ev) => ev.hash(hasher),
            Value::Nil => 0.hash(hasher)
        }
        hasher.finish();
    }
}

impl Eq for Value {}

#[derive(Debug, Hash, Eq, PartialOrd, PartialEq)]
pub struct InstanceObj {
    pub typ: Box<Value>,
    pub fields: Vec<Value>,
    pub methods: Vec<Value>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, PartialOrd)]
pub struct EnumVariantObj {
    pub enum_name: String,
    pub name: String,
    pub idx: usize,
    pub methods: Vec<Value>,
    pub arity: usize,
    pub values: Option<Vec<Value>>,
}

#[derive(Debug)]
pub struct NativeInstanceObj {
    pub typ: TypeValue,
    pub inst: Box<dyn NativeValue>,
    pub methods: Vec<Value>,
}

impl NativeInstanceObj {
    pub fn as_array(&self) -> Option<&Array> {
        self.inst.downcast_ref::<Array>()
    }
}

#[derive(Debug)]
pub enum Obj {
    StringObj(String),
    SetObj(HashSet<Value>),
    TupleObj(Vec<Value>),
    MapObj(HashMap<Value, Value>),
    InstanceObj(InstanceObj),
    EnumVariantObj(EnumVariantObj),
    NativeInstanceObj(NativeInstanceObj),
}

impl Display for Obj {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Obj::StringObj(value) => write!(f, "\"{}\"", value),
            Obj::SetObj(value) => {
                let items = value.iter().map(|v| format!("{}", v)).join(", ");
                write!(f, "#{{{}}}", items)
            }
            Obj::TupleObj(value) => {
                let items = value.iter().map(|v| format!("{}", v)).join(", ");
                write!(f, "({})", items)
            }
            Obj::MapObj(map) => {
                let fields = map.iter().map(|(k, v)| format!("{}: {}", k, v)).join(", ");
                write!(f, "{{ {} }}", fields)
            }
            Obj::InstanceObj(inst) => {
                match &*inst.typ {
                    Value::Type(TypeValue { name, .. }) => write!(f, "<instance {}>", name),
                    _ => unreachable!("Shouldn't have instances of non-struct types")
                }
            }
            Obj::EnumVariantObj(EnumVariantObj { enum_name, name, values, .. }) => {
                match values {
                    None => write!(f, "{}.{}", enum_name, name),
                    Some(values) => {
                        let values = values.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ");
                        write!(f, "{}.{}({})", enum_name, name, values)
                    }
                }
            }
            Obj::NativeInstanceObj(inst) => {
                let TypeValue { name, .. } = &inst.typ;
                write!(f, "<instance {}>", name)
            }
        }
    }
}

impl PartialOrd for Obj {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Obj::StringObj(v1), Obj::StringObj(v2)) => Some(v1.cmp(v2)),
            (Obj::TupleObj(v1), Obj::TupleObj(v2)) => {
                if v1.len() < v2.len() {
                    Some(Ordering::Less)
                } else if v1.len() > v2.len() {
                    Some(Ordering::Greater)
                } else {
                    for (i1, i2) in v1.iter().zip(v2.iter()) {
                        if let Some(o) = i1.partial_cmp(&i2) {
                            if o != Ordering::Equal {
                                return Some(o);
                            }
                        }
                    }
                    Some(Ordering::Equal)
                }
            }
            (Obj::SetObj(s1), Obj::SetObj(s2)) => {
                if s1.len() < s2.len() {
                    Some(Ordering::Less)
                } else if s1.len() > s2.len() {
                    Some(Ordering::Greater)
                } else if s1.difference(&s2).count() == 0 {
                    Some(Ordering::Equal)
                } else {
                    Some(Ordering::Less)
                }
            }
            (Obj::EnumVariantObj(evv1), Obj::EnumVariantObj(evv2)) => {
                match evv1.idx.cmp(&evv2.idx) {
                    Ordering::Equal => {}
                    v @ _ => return Some(v)
                };
                match evv1.enum_name.cmp(&evv2.enum_name) {
                    Ordering::Equal => {}
                    v @ _ => return Some(v)
                };
                if evv1.arity > 0 { // evv2.arity should also be 0
                    let evv1_values = evv1.values.as_ref().expect("If it has an arity > 0, it should have values");
                    let evv2_values = evv2.values.as_ref().expect("If it has an arity > 0, it should have values");
                    for (v1, v2) in evv1_values.iter().zip(evv2_values.iter()) {
                        if let Some(o) = v1.partial_cmp(&v2) {
                            if o != Ordering::Equal {
                                return Some(o);
                            }
                        }
                    }
                }
                Some(Ordering::Equal)
            }
            (Obj::NativeInstanceObj(v1), Obj::NativeInstanceObj(v2)) => {
                if v1.inst.is_equal(&v2.inst) {
                    Some(Ordering::Equal)
                } else {
                    Some(Ordering::Less)
                }
            }
            (_, _) => None
        }
    }
}

impl PartialEq for Obj {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Obj::StringObj(v1), Obj::StringObj(v2)) => v1.eq(v2),
            (Obj::SetObj(v1), Obj::SetObj(v2)) => v1.eq(v2),
            (Obj::TupleObj(v1), Obj::TupleObj(v2)) => v1.eq(v2),
            (Obj::MapObj(v1), Obj::MapObj(v2)) => v1.eq(v2),
            (Obj::InstanceObj(v1), Obj::InstanceObj(v2)) => v1.eq(v2),
            (Obj::EnumVariantObj(v1), Obj::EnumVariantObj(v2)) => v1.eq(v2),
            (Obj::NativeInstanceObj(v1), Obj::NativeInstanceObj(v2)) => v1.inst.is_equal(&v2.inst),
            _ => false
        }
    }
}

impl Eq for Obj {}

impl Hash for Obj {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        match self {
            Obj::StringObj(s) => s.hash(hasher),
            Obj::TupleObj(a) => a.hash(hasher),
            Obj::SetObj(s) => {
                for item in s {
                    item.hash(hasher);
                }
            }
            Obj::MapObj(m) => {
                for (k, v) in m {
                    k.hash(hasher);
                    v.hash(hasher);
                }
            }
            Obj::InstanceObj(i) => {
                i.typ.hash(hasher);
                i.fields.hash(hasher);
            }
            Obj::EnumVariantObj(ev) => ev.hash(hasher),
            Obj::NativeInstanceObj(i) => i.inst.hash(hasher),
        }
        hasher.finish();
    }
}
