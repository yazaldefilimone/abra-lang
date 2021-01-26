extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate wasm_bindgen;
extern crate wasm_bindgen_futures;
extern crate js_sys;
extern crate futures;

mod js_value;

use crate::js_value::module::JsModule;
use crate::js_value::error::JsWrappedError;
use futures::Future;
use serde::ser::{Serializer, SerializeSeq};
use serde::Serialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use abra_core::builtins::native_fns::NativeFn;
use abra_core::{Error, typecheck, compile, compile_and_disassemble};
use abra_core::vm::value::{Value, FnValue, ClosureValue, TypeValue, EnumValue, NativeInstanceObj};
use abra_core::vm::vm::{VMContext, VM};
use abra_core::vm::compiler::Module;
use abra_core::common::display_error::DisplayError;

pub struct RunResultValue(Option<Value>);

impl Serialize for RunResultValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        use serde::ser::SerializeMap;

        if self.0.is_none() {
            return serializer.serialize_none();
        }

        match &self.0.as_ref().unwrap() {
            Value::Nil => serializer.serialize_none(),
            Value::Int(val) => serializer.serialize_i64(*val),
            Value::Float(val) => serializer.serialize_f64(*val),
            Value::Bool(val) => serializer.serialize_bool(*val),
            Value::Str(val) => serializer.serialize_str(val),
            Value::StringObj(o) => {
                serializer.serialize_str(&*o.borrow()._inner)
            }
            Value::ArrayObj(o) => {
                let arr = &*o.borrow();
                let array = &arr._inner;
                let mut arr = serializer.serialize_seq(Some((*array).len()))?;
                array.iter().for_each(|val| {
                    arr.serialize_element(&RunResultValue(Some((*val).clone()))).unwrap();
                });
                arr.end()
            }
            Value::TupleObj(o) => {
                let tup = &*o.borrow();
                let mut arr = serializer.serialize_seq(Some((*tup).len()))?;
                tup.iter().for_each(|val| {
                    arr.serialize_element(&RunResultValue(Some((*val).clone()))).unwrap();
                });
                arr.end()
            }
            Value::SetObj(o) => {
                let set = &*o.borrow();
                let items = &set._inner;
                let mut set = serializer.serialize_seq(Some((*items).len()))?;
                items.iter().for_each(|val| {
                    set.serialize_element(&RunResultValue(Some((*val).clone()))).unwrap();
                });
                set.end()
            }
            Value::MapObj(o) => {
                let map = &*o.borrow();
                let map = &map._inner;
                let mut obj = serializer.serialize_map(Some((*map).len()))?;
                map.into_iter().for_each(|(key, val)| {
                    obj.serialize_entry(&RunResultValue(Some(key.clone())), &RunResultValue(Some(val.clone()))).unwrap();
                });
                obj.end()
            }
            Value::InstanceObj(o) => {
                let inst = &*o.borrow();

                let fields = &inst.fields;
                let mut arr = serializer.serialize_seq(Some(fields.len()))?;
                fields.into_iter().for_each(|val| {
                    arr.serialize_element(&RunResultValue(Some((*val).clone()))).unwrap();
                });
                arr.end()
            }
            Value::NativeInstanceObj(o) => {
                let NativeInstanceObj { typ, inst } = &*o.borrow();

                let mut obj = serializer.serialize_map(Some(typ.fields.len()))?;

                for (field_name, field_value) in typ.fields.iter().zip(inst.get_field_values()) {
                    obj.serialize_entry(field_name, &RunResultValue(Some(field_value)))?;
                }

                obj.end()
            }
            Value::EnumVariantObj(o) => serializer.serialize_str(&*o.borrow().name),
            Value::Fn(FnValue { name, .. }) => serializer.serialize_str(name),
            Value::Closure(ClosureValue { name, .. }) => serializer.serialize_str(name),
            Value::NativeFn(NativeFn { name, .. }) => serializer.serialize_str(name),
            Value::Type(TypeValue { name, .. }) => serializer.serialize_str(name),
            Value::Enum(EnumValue { name, .. }) => serializer.serialize_str(name),
        }
    }
}

pub struct RunResult(Result<Option<Value>, Error>, String);

impl Serialize for RunResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        use serde::ser::SerializeMap;

        let mut obj = serializer.serialize_map(Some(2))?;

        match &self.0 {
            Ok(value) => {
                obj.serialize_entry("success", &true)?;
                obj.serialize_entry("data", &RunResultValue((*value).clone()))?;
            }
            Err(error) => {
                obj.serialize_entry("success", &false)?;
                obj.serialize_entry("error", &JsWrappedError(&error, &self.1))?;
                obj.serialize_entry("errorMessage", &error.get_message(&self.1))?;
            }
        };

        obj.end()
    }
}

pub struct CompileResult(Result<Module, Error>, String);

impl Serialize for CompileResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        use serde::ser::SerializeMap;

        match &self.0 {
            Ok(module) => {
                let mut obj = serializer.serialize_map(Some(2))?;
                obj.serialize_entry("success", &true)?;
                obj.serialize_entry("module", &JsModule(module))?;
                obj.end()
            }
            Err(error) => {
                let mut obj = serializer.serialize_map(Some(2))?;
                obj.serialize_entry("success", &false)?;
                obj.serialize_entry("error", &JsWrappedError(error, &self.1))?;
                obj.serialize_entry("errorMessage", &error.get_message(&self.1))?;
                obj.end()
            }
        }
    }
}

pub struct TypecheckedResult(Result<(), Error>, String);

impl Serialize for TypecheckedResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        use serde::ser::SerializeMap;

        match &self.0 {
            Ok(_) => {
                let mut obj = serializer.serialize_map(Some(1))?;
                obj.serialize_entry("success", &true)?;
                obj.end()
            }
            Err(error) => {
                let mut obj = serializer.serialize_map(Some(2))?;
                obj.serialize_entry("success", &false)?;
                obj.serialize_entry("error", &JsWrappedError(error, &self.1))?;
                obj.serialize_entry("errorMessage", &error.get_message(&self.1))?;
                obj.end()
            }
        }
    }
}

pub struct DisassembleResult(Result<String, Error>, String);

impl Serialize for DisassembleResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        use serde::ser::SerializeMap;

        match &self.0 {
            Ok(result) => {
                let mut obj = serializer.serialize_map(Some(2))?;
                obj.serialize_entry("success", &true)?;
                obj.serialize_entry("disassembled", result)?;
                obj.end()
            }
            Err(error) => {
                let mut obj = serializer.serialize_map(Some(2))?;
                obj.serialize_entry("success", &false)?;
                obj.serialize_entry("error", &JsWrappedError(error, &self.1))?;
                obj.serialize_entry("errorMessage", &error.get_message(&self.1))?;
                obj.end()
            }
        }
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = __abra_func__println)]
    fn println(s: &str);
}

#[wasm_bindgen(js_name = disassemble)]
pub fn disassemble(input: &str) -> JsValue {
    let result = compile_and_disassemble(&input.to_string());
    let disassemble_result = DisassembleResult(result, input.to_string());
    JsValue::from_serde(&disassemble_result)
        .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = typecheck)]
pub fn typecheck_input(input: &str) -> JsValue {
    let result = typecheck(&input.to_string())
        .map(|_| ());
    let typecheck_result = TypecheckedResult(result, input.to_string());
    JsValue::from_serde(&typecheck_result)
        .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = compile)]
pub fn parse_typecheck_and_compile(input: &str) -> JsValue {
    let result = compile(&input.to_string())
        .map(|(module, _)| module);
    let compile_result = CompileResult(result, input.to_string());
    JsValue::from_serde(&compile_result)
        .unwrap_or(JsValue::NULL)
}

fn compile_and_run(input: String, ctx: VMContext) -> Result<Option<Value>, Error> {
    let (module, _) = compile(&input)?;
    let mut vm = VM::new(module, ctx);
    match vm.run() {
        Ok(Some(v)) => Ok(Some(v)),
        Ok(None) => Ok(None),
        Err(e) => Err(Error::InterpretError(e)),
    }
}

#[wasm_bindgen(js_name = runSync)]
pub fn run(input: &str) -> JsValue {
    let ctx = VMContext {
        print: |input| println(input)
    };

    let result = compile_and_run(input.to_string(), ctx);
    let run_result = RunResult(result, input.to_string().clone());
    JsValue::from_serde(&run_result)
        .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = runAsync)]
pub fn run_async(input: &str) -> js_sys::Promise {
    let ctx = VMContext {
        print: |input| println(input)
    };

    let future = futures::future::ok(input.to_string())
        .and_then(move |input| {
            let result = compile_and_run(input.to_string(), ctx);
            let run_result = RunResult(result, input.to_string());
            let val = JsValue::from_serde(&run_result)
                .unwrap_or(JsValue::NULL);
            Ok(val)
        });
    future_to_promise(future)
}
