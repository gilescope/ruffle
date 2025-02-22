//! `String` impl

use crate::avm2::activation::Activation;
use crate::avm2::class::{Class, ClassAttributes};
use crate::avm2::method::{Method, NativeMethodImpl};
use crate::avm2::names::{Namespace, QName};
use crate::avm2::object::{primitive_allocator, Object, TObject};
use crate::avm2::value::Value;
use crate::avm2::ArrayObject;
use crate::avm2::Error;
use crate::string::utils as string_utils;
use crate::string::AvmString;
use gc_arena::{GcCell, MutationContext};
use std::iter;

/// Implements `String`'s instance initializer.
pub fn instance_init<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    if let Some(this) = this {
        activation.super_init(this, &[])?;

        if let Some(mut value) = this.as_primitive_mut(activation.context.gc_context) {
            if !matches!(*value, Value::String(_)) {
                *value = args
                    .get(0)
                    .unwrap_or(&Value::String("".into()))
                    .coerce_to_string(activation)?
                    .into();
            }
        }
    }

    Ok(Value::Undefined)
}

/// Implements `String`'s class initializer.
pub fn class_init<'gc>(
    _activation: &mut Activation<'_, 'gc, '_>,
    _this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    Ok(Value::Undefined)
}

/// Implements `length` property's getter
fn length<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    if let Some(this) = this {
        if let Value::String(s) = this.value_of(activation.context.gc_context)? {
            return Ok(s.encode_utf16().count().into());
        }
    }

    Ok(Value::Undefined)
}

/// Implements `String.charAt`
fn char_at<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    if let Some(this) = this {
        if let Value::String(s) = this.value_of(activation.context.gc_context)? {
            // This function takes Number, so if we use coerce_to_i32 instead of coerce_to_number, the value may overflow.
            let n = args
                .get(0)
                .unwrap_or(&Value::Number(0.0))
                .coerce_to_number(activation)?;
            if n < 0.0 {
                return Ok("".into());
            }

            let index = if !n.is_nan() { n as usize } else { 0 };
            let ret = s
                .encode_utf16()
                .nth(index)
                .map(|c| string_utils::utf16_code_unit_to_char(c).to_string())
                .unwrap_or_default();
            return Ok(AvmString::new(activation.context.gc_context, ret).into());
        }
    }

    Ok(Value::Undefined)
}

/// Implements `String.charCodeAt`
fn char_code_at<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    if let Some(this) = this {
        if let Value::String(s) = this.value_of(activation.context.gc_context)? {
            // This function takes Number, so if we use coerce_to_i32 instead of coerce_to_number, the value may overflow.
            let n = args
                .get(0)
                .unwrap_or(&Value::Number(0.0))
                .coerce_to_number(activation)?;
            if n < 0.0 {
                return Ok(f64::NAN.into());
            }

            let index = if !n.is_nan() { n as usize } else { 0 };
            let ret = s
                .encode_utf16()
                .nth(index)
                .map(f64::from)
                .unwrap_or(f64::NAN);
            return Ok(ret.into());
        }
    }

    Ok(Value::Undefined)
}

/// Implements `String.split`
fn split<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error> {
    if let Some(this) = this {
        let delimiter = args.get(0).unwrap_or(&Value::Undefined);
        if matches!(delimiter, Value::Undefined) {
            let this = Value::from(this);
            return Ok(
                ArrayObject::from_storage(activation, iter::once(this).collect())
                    .unwrap()
                    .into(),
            );
        }
        if delimiter
            .coerce_to_object(activation)?
            .as_regexp()
            .is_some()
        {
            log::warn!("string.split(regex) - not implemented");
        }
        let this = Value::from(this).coerce_to_string(activation)?;
        let delimiter = delimiter.coerce_to_string(activation)?;
        let limit = match args.get(1).unwrap_or(&Value::Undefined) {
            Value::Undefined => usize::MAX,
            limit => limit.coerce_to_i32(activation)?.max(0) as usize,
        };
        if delimiter.is_empty() {
            // When using an empty delimiter, Rust's str::split adds an extra beginning and trailing item, but Flash does not.
            // e.g., split("foo", "") returns ["", "f", "o", "o", ""] in Rust but ["f, "o", "o"] in Flash.
            // Special case this to match Flash's behavior.
            return Ok(ArrayObject::from_storage(
                activation,
                this.chars()
                    .take(limit)
                    .map(|c| AvmString::new(activation.context.gc_context, c.to_string()))
                    .collect(),
            )
            .unwrap()
            .into());
        } else {
            return Ok(ArrayObject::from_storage(
                activation,
                this.split(delimiter.as_ref())
                    .take(limit)
                    .map(|c| AvmString::new(activation.context.gc_context, c.to_string()))
                    .collect(),
            )
            .unwrap()
            .into());
        }
    }
    Ok(Value::Undefined)
}

/// Construct `String`'s class.
pub fn create_class<'gc>(mc: MutationContext<'gc, '_>) -> GcCell<'gc, Class<'gc>> {
    let class = Class::new(
        QName::new(Namespace::public(), "String"),
        Some(QName::new(Namespace::public(), "Object").into()),
        Method::from_builtin(instance_init, "<String instance initializer>", mc),
        Method::from_builtin(class_init, "<String class initializer>", mc),
        mc,
    );

    let mut write = class.write(mc);
    write.set_attributes(ClassAttributes::FINAL | ClassAttributes::SEALED);
    write.set_instance_allocator(primitive_allocator);

    const PUBLIC_INSTANCE_PROPERTIES: &[(
        &str,
        Option<NativeMethodImpl>,
        Option<NativeMethodImpl>,
    )] = &[("length", Some(length), None)];
    write.define_public_builtin_instance_properties(mc, PUBLIC_INSTANCE_PROPERTIES);

    const AS3_INSTANCE_METHODS: &[(&str, NativeMethodImpl)] = &[
        ("charAt", char_at),
        ("charCodeAt", char_code_at),
        ("split", split),
    ];
    write.define_as3_builtin_instance_methods(mc, AS3_INSTANCE_METHODS);

    class
}
