#![feature(associated_type_defaults)]
#![allow(non_snake_case)]
#![feature(utf16_extra)]
#![allow(clippy::needless_lifetimes)]

use std::any::TypeId;
use std::sync::OnceLock;
use jni::errors::Exception;
use jni::JNIEnv;
use jni::objects::{JBooleanArray, JByteArray, JCharArray, JDoubleArray, JFloatArray, JIntArray, JLongArray, JObject, JObjectArray, JShortArray, JString, JValueOwned, ReleaseMode};
use jni::strings::JavaStr;
use jni::sys::{jboolean, jbyte, jchar, jdouble, jfloat, jint, jlong, jshort, jsize};

use jni_util::map_jni_error;

use crate::interop::JavaChar;

/// Module for proc_macro re-exports, from instant-coffee-proc-macro
pub mod proc_macro {
    pub use instant_coffee_proc_macro::JavaType;
    pub use instant_coffee_proc_macro::jmodule;
    pub use instant_coffee_proc_macro::jmodule_package;
    pub use instant_coffee_proc_macro::jmodule_methods;
}

pub mod jni_util;

pub mod interop;

pub mod codegen;

/// Trait describing a mapping between a JNI array type, and a [`JavaType`] 'T'
///
/// Implementations for boolean/byte/short/int/long/float/double/char and their respective rust types are provided, as well as a blanket implementation for all object arrays
pub trait JniArray<'local, T: JavaType>: From<JObject<'local>> + AsRef<JObject<'local>> {
    /// Null value for this array, returned when exceptions are thrown
    fn EXCEPTION_NULL() -> Self {
        Self::from(JObject::null())
    }

    /// Convert this array from JNI array type to a boxed slice of rust type
    fn from_jni(jni_value: Self, env: &mut JNIEnv<'local>) -> Result<Box<[T]>, Option<Exception>>;
    /// Convert this array from rust boxed slice type to a JNI array type
    #[allow(clippy::wrong_self_convention)] // This function acts on the Box<[T]> JavaType, and mirrors the JavaType::into_jni function name
    fn into_jni(input: Box<[T]>, env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>>;
}

impl<'local> JniArray<'local, bool> for JBooleanArray<'local> {
    fn from_jni(jni_value: Self, env: &mut JNIEnv<'local>) -> Result<Box<[bool]>, Option<Exception>> {
        Ok(
            unsafe { env.get_array_elements(&jni_value, ReleaseMode::NoCopyBack) }
                .map_err(map_jni_error)?
                .iter()
                .map(|jboolean| *jboolean != 0)
                .collect::<Box<[bool]>>()
        )
    }

    fn into_jni(input: Box<[bool]>, env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        let array = env.new_boolean_array(input.len() as jsize)
            .map_err(map_jni_error)?;

        let jslice = input.iter().map(|bool| *bool as jboolean).collect::<Vec<jboolean>>();

        env.set_boolean_array_region(&array, 0, &*jslice).map_err(map_jni_error)?;

        Ok(array)
    }
}

impl<'local> JniArray<'local, u8> for JByteArray<'local> {
    fn from_jni(jni_value: Self, env: &mut JNIEnv<'local>) -> Result<Box<[u8]>, Option<Exception>> {
        Ok(
            unsafe { env.get_array_elements(&jni_value, ReleaseMode::NoCopyBack) }
                .map_err(map_jni_error)?
                .iter()
                .map(|byte| *byte as u8)
                .collect::<Box<[u8]>>()
        )
    }

    fn into_jni(input: Box<[u8]>, env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        let array = env.new_byte_array(input.len() as jsize)
            .map_err(map_jni_error)?;

        // if this fails, jbyte is no longer identical to i8, and the following pointer cast is unsafe
        assert_eq!(TypeId::of::<i8>(), TypeId::of::<jbyte>());

        let slice: &[jbyte] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const jbyte, input.len()) };
        env.set_byte_array_region(&array, 0, slice).map_err(map_jni_error)?;

        Ok(array)
    }
}

impl<'local> JniArray<'local, i8> for JByteArray<'local> {
    fn from_jni(jni_value: Self, env: &mut JNIEnv<'local>) -> Result<Box<[i8]>, Option<Exception>> {
        Ok(
            unsafe { env.get_array_elements(&jni_value, ReleaseMode::NoCopyBack) }
                .map_err(map_jni_error)?
                .iter()
                .map(|byte| *byte as i8)
                .collect::<Box<[i8]>>()
        )
    }

    fn into_jni(input: Box<[i8]>, env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        let array = env.new_byte_array(input.len() as jsize)
            .map_err(map_jni_error)?;

        // if this fails, jbyte is no longer identical to i8, and the following pointer cast is unsafe
        assert_eq!(TypeId::of::<i8>(), TypeId::of::<jbyte>());

        let slice: &[jbyte] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const jbyte, input.len()) };
        env.set_byte_array_region(&array, 0, slice).map_err(map_jni_error)?;

        Ok(array)
    }
}

impl<'local> JniArray<'local, u16> for JShortArray<'local> {
    fn from_jni(jni_value: Self, env: &mut JNIEnv<'local>) -> Result<Box<[u16]>, Option<Exception>> {
        Ok(
            unsafe { env.get_array_elements(&jni_value, ReleaseMode::NoCopyBack) }
                .map_err(map_jni_error)?
                .iter()
                .map(|short| *short as u16)
                .collect::<Box<[u16]>>()
        )
    }

    fn into_jni(input: Box<[u16]>, env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        let array = env.new_short_array(input.len() as jsize)
            .map_err(map_jni_error)?;

        // if this fails, jshort is no longer identical to i16, and the following pointer cast is unsafe
        assert_eq!(TypeId::of::<i16>(), TypeId::of::<jshort>());

        let slice: &[jshort] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const jshort, input.len()) };
        env.set_short_array_region(&array, 0, slice).map_err(map_jni_error)?;

        Ok(array)
    }
}

impl<'local> JniArray<'local, i16> for JShortArray<'local> {
    fn from_jni(jni_value: Self, env: &mut JNIEnv<'local>) -> Result<Box<[i16]>, Option<Exception>> {
        Ok(
            unsafe { env.get_array_elements(&jni_value, ReleaseMode::NoCopyBack) }
                .map_err(map_jni_error)?
                .iter()
                .map(|short| *short as i16)
                .collect::<Box<[i16]>>()
        )
    }

    fn into_jni(input: Box<[i16]>, env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        let array = env.new_short_array(input.len() as jsize)
            .map_err(map_jni_error)?;

        // if this fails, jshort is no longer identical to i16, and the following pointer cast is unsafe
        assert_eq!(TypeId::of::<i16>(), TypeId::of::<jshort>());

        let slice: &[jshort] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const jshort, input.len()) };
        env.set_short_array_region(&array, 0, slice).map_err(map_jni_error)?;

        Ok(array)
    }
}

impl<'local> JniArray<'local, u32> for JIntArray<'local> {
    fn from_jni(jni_value: Self, env: &mut JNIEnv<'local>) -> Result<Box<[u32]>, Option<Exception>> {
        Ok(
            unsafe { env.get_array_elements(&jni_value, ReleaseMode::NoCopyBack) }
                .map_err(map_jni_error)?
                .iter()
                .map(|int| *int as u32)
                .collect::<Box<[u32]>>()
        )
    }

    fn into_jni(input: Box<[u32]>, env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        let array = env.new_int_array(input.len() as jsize)
            .map_err(map_jni_error)?;

        // if this fails, jint is no longer identical to i32, and the following pointer cast is unsafe
        assert_eq!(TypeId::of::<i32>(), TypeId::of::<jint>());

        let slice: &[jint] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const jint, input.len()) };
        env.set_int_array_region(&array, 0, slice).map_err(map_jni_error)?;

        Ok(array)
    }
}

impl<'local> JniArray<'local, i32> for JIntArray<'local> {
    fn from_jni(jni_value: Self, env: &mut JNIEnv<'local>) -> Result<Box<[i32]>, Option<Exception>> {
        Ok(
            unsafe { env.get_array_elements(&jni_value, ReleaseMode::NoCopyBack) }
                .map_err(map_jni_error)?
                .iter()
                .map(|int| *int as i32)
                .collect::<Box<[i32]>>()
        )
    }

    fn into_jni(input: Box<[i32]>, env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        let array = env.new_int_array(input.len() as jsize)
            .map_err(map_jni_error)?;

        // if this fails, jint is no longer identical to i32, and the following pointer cast is unsafe
        assert_eq!(TypeId::of::<i32>(), TypeId::of::<jint>());

        let slice: &[jint] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const jint, input.len()) };
        env.set_int_array_region(&array, 0, slice).map_err(map_jni_error)?;

        Ok(array)
    }
}

impl<'local> JniArray<'local, u64> for JLongArray<'local> {
    fn from_jni(jni_value: Self, env: &mut JNIEnv<'local>) -> Result<Box<[u64]>, Option<Exception>> {
        Ok(
            unsafe { env.get_array_elements(&jni_value, ReleaseMode::NoCopyBack) }
                .map_err(map_jni_error)?
                .iter()
                .map(|long| *long as u64)
                .collect::<Box<[u64]>>()
        )
    }

    fn into_jni(input: Box<[u64]>, env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        let array = env.new_long_array(input.len() as jsize)
            .map_err(map_jni_error)?;

        // if this fails, jlong is no longer identical to i64, and the following pointer cast is unsafe
        assert_eq!(TypeId::of::<i64>(), TypeId::of::<jlong>());

        let slice: &[jlong] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const jlong, input.len()) };
        env.set_long_array_region(&array, 0, slice).map_err(map_jni_error)?;

        Ok(array)
    }
}

impl<'local> JniArray<'local, i64> for JLongArray<'local> {
    fn from_jni(jni_value: Self, env: &mut JNIEnv<'local>) -> Result<Box<[i64]>, Option<Exception>> {
        Ok(
            unsafe { env.get_array_elements(&jni_value, ReleaseMode::NoCopyBack) }
                .map_err(map_jni_error)?
                .iter()
                .map(|long| *long as i64)
                .collect::<Box<[i64]>>()
        )
    }

    fn into_jni(input: Box<[i64]>, env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        let array = env.new_long_array(input.len() as jsize)
            .map_err(map_jni_error)?;

        // if this fails, jlong is no longer identical to i64, and the following pointer cast is unsafe
        assert_eq!(TypeId::of::<i64>(), TypeId::of::<jlong>());

        let slice: &[jlong] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const jlong, input.len()) };
        env.set_long_array_region(&array, 0, slice).map_err(map_jni_error)?;

        Ok(array)
    }
}

impl<'local> JniArray<'local, f32> for JFloatArray<'local> {
    fn from_jni(jni_value: Self, env: &mut JNIEnv<'local>) -> Result<Box<[f32]>, Option<Exception>> {
        Ok(
            unsafe { env.get_array_elements(&jni_value, ReleaseMode::NoCopyBack) }
                .map_err(map_jni_error)?
                .iter()
                .map(|float| *float as f32)
                .collect::<Box<[f32]>>()
        )
    }

    fn into_jni(input: Box<[f32]>, env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        let array = env.new_float_array(input.len() as jsize)
            .map_err(map_jni_error)?;

        // if this fails, jfloat is no longer identical to f32, and the following pointer cast is unsafe
        assert_eq!(TypeId::of::<f32>(), TypeId::of::<jfloat>());

        let slice: &[jfloat] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const jfloat, input.len()) };
        env.set_float_array_region(&array, 0, slice).map_err(map_jni_error)?;

        Ok(array)
    }
}

impl<'local> JniArray<'local, f64> for JDoubleArray<'local> {
    fn from_jni(jni_value: Self, env: &mut JNIEnv<'local>) -> Result<Box<[f64]>, Option<Exception>> {
        Ok(
            unsafe { env.get_array_elements(&jni_value, ReleaseMode::NoCopyBack) }
                .map_err(map_jni_error)?
                .iter()
                .map(|double| *double as f64)
                .collect::<Box<[f64]>>()
        )
    }

    fn into_jni(input: Box<[f64]>, env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        let array = env.new_double_array(input.len() as jsize)
            .map_err(map_jni_error)?;

        // if this fails, jdouble is no longer identical to f64, and the following pointer cast is unsafe
        assert_eq!(TypeId::of::<f64>(), TypeId::of::<jdouble>());

        let slice: &[jdouble] = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const jdouble, input.len()) };
        env.set_double_array_region(&array, 0, slice).map_err(map_jni_error)?;

        Ok(array)
    }
}

impl<'local> JniArray<'local, JavaChar> for JCharArray<'local> {
    fn from_jni(jni_value: Self, env: &mut JNIEnv<'local>) -> Result<Box<[JavaChar]>, Option<Exception>> {
        Ok(
            unsafe { env.get_array_elements(&jni_value, ReleaseMode::NoCopyBack) }
                .map_err(map_jni_error)?
                .iter()
                .map(|char| JavaChar(*char))
                .collect::<Box<[JavaChar]>>()
        )
    }

    fn into_jni(input: Box<[JavaChar]>, env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        let array = env.new_char_array(input.len() as jsize)
            .map_err(map_jni_error)?;

        let jslice = input.iter().map(|char| char.0 as jchar).collect::<Vec<jchar>>();
        env.set_char_array_region(&array, 0, &*jslice).map_err(map_jni_error)?;

        Ok(array)
    }
}

impl<'local, T: JavaType<JniType<'local>: From<JObject<'local>> + AsRef<JObject<'local>>>> JniArray<'local, T> for JObjectArray<'local> {
    fn from_jni(jni_value: Self, env: &mut JNIEnv<'local>) -> Result<Box<[T]>, Option<Exception>> {
        let mut buffer = Vec::new();
        let array_size = env.get_array_length(&jni_value).map_err(map_jni_error)?;

        for i in 0..array_size {
            let value = env.get_object_array_element(&jni_value, i).map_err(map_jni_error)?;

            buffer.push(T::from_jni(value.into(), env)?);
        }

        Ok(buffer.into_boxed_slice())
    }

    fn into_jni(input: Box<[T]>, env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        let array = env.new_object_array(input.len() as jsize, T::JVM_PARAM_SIGNATURE(), JObject::null()).map_err(map_jni_error)?;

        for (idx, element) in input.into_vec().into_iter().enumerate() {
            let jelement = element.into_jni(env)?;
            env.set_object_array_element(&array, idx as jsize, jelement.as_ref()).map_err(map_jni_error)?;
        }

        Ok(array)
    }
}

/// Main trait for types with a Java equivalent
pub trait JavaType: Sized {
    /// Jni equivalent to this type; Used as type in FFI functions
    type JniType<'local>;
    /// Jni array type that can store Self::JniType
    type ArrayType<'local>: JniArray<'local, Self>;

    /// Fully qualified java name of this type, such as "java.lang.Object"
    fn QUALIFIED_NAME() -> &'static str;

    /// JVM "internal" type signature, such as "Ljava/lang/Object;"
    fn JVM_PARAM_SIGNATURE() -> &'static str;

    /// 'Null' value to return to JNI in the event of exceptions. For objects this is a null pointer, for numerical types it is zero, for booleans it is false
    fn EXCEPTION_NULL<'local>() -> Self::JniType<'local>;

    /// Convert from JNI type to rust type
    fn from_jni<'local>(jni_value: Self::JniType<'local>, env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>>;

    /// Convert from rust type to JNI type
    fn into_jni<'local>(self, env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>>;
    /// Convert from [`JValueOwned`] (a java primitive or object value) to JNI type
    fn from_jvalue<'local>(jvalue: JValueOwned<'local>, env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>>;
}

/// Java boolean = rust bool
impl JavaType for bool {
    type JniType<'local> = jboolean;
    type ArrayType<'local> = JBooleanArray<'local>;

    fn QUALIFIED_NAME() -> &'static str { "boolean" }

    fn JVM_PARAM_SIGNATURE() -> &'static str { "Z" }

    fn EXCEPTION_NULL<'local>() -> Self::JniType<'local> { false as jboolean }

    fn from_jni<'local>(jni_value: Self::JniType<'local>, _env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        Ok(jni_value != 0)  // Boolean stored as integer type
    }

    fn into_jni<'local>(self, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        Ok(self as <Self as JavaType>::JniType<'local>)  // cast boolean to integer type
    }

    fn from_jvalue<'local>(jvalue: JValueOwned<'local>, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        match jvalue {
            JValueOwned::Bool(boolean) => Ok(boolean),
            _ => Err(Some(Exception { class: "java/lang/ClassCastException".to_string(), msg: format!("{} cannot be cast to {}", jvalue.type_name(), <Self as JavaType>::QUALIFIED_NAME()) }))
        }
    }
}

/// Java byte = rust i8
impl JavaType for i8 {
    type JniType<'local> = jbyte;
    type ArrayType<'local> = JByteArray<'local>;

    fn QUALIFIED_NAME() -> &'static str { "byte" }

    fn JVM_PARAM_SIGNATURE() -> &'static str { "B" }

    fn EXCEPTION_NULL<'local>() -> Self::JniType<'local> { 0 }

    fn from_jni<'local>(jni_value: Self::JniType<'local>, _env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        Ok(jni_value as Self)
    }

    fn into_jni<'local>(self, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        Ok(self as <Self as JavaType>::JniType<'local>)  // identical types
    }

    fn from_jvalue<'local>(jvalue: JValueOwned<'local>, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        match jvalue {
            JValueOwned::Byte(byte) => Ok(byte),
            _ => Err(Some(Exception { class: "java/lang/ClassCastException".to_string(), msg: format!("{} cannot be cast to {}", jvalue.type_name(), <Self as JavaType>::QUALIFIED_NAME()) }))
        }
    }
}

/// Java byte = rust u8 (byte interpreted unsigned)
impl JavaType for u8 {
    type JniType<'local> = jbyte;
    type ArrayType<'local> = JByteArray<'local>;

    fn QUALIFIED_NAME() -> &'static str { "byte" }

    fn JVM_PARAM_SIGNATURE() -> &'static str { "B" }

    fn EXCEPTION_NULL<'local>() -> Self::JniType<'local> {
        0
    }

    fn from_jni<'local>(jni_value: Self::JniType<'local>, _env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        Ok(jni_value as Self)
    }

    fn into_jni<'local>(self, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        Ok(self as <Self as JavaType>::JniType<'local>)  // identical types
    }

    fn from_jvalue<'local>(jvalue: JValueOwned<'local>, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        match jvalue {
            JValueOwned::Byte(byte) => Ok(byte),
            _ => Err(Some(Exception { class: "java/lang/ClassCastException".to_string(), msg: format!("{} cannot be cast to {}", jvalue.type_name(), <Self as JavaType>::QUALIFIED_NAME()) }))
        }
    }
}

/// Java short = rust i16
impl JavaType for i16 {
    type JniType<'local> = jshort;
    type ArrayType<'local> = JShortArray<'local>;

    fn QUALIFIED_NAME() -> &'static str { "short" }

    fn JVM_PARAM_SIGNATURE() -> &'static str { "S" }

    fn EXCEPTION_NULL<'local>() -> Self::JniType<'local> { 0 }

    fn from_jni<'local>(jni_value: Self::JniType<'local>, _env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        Ok(jni_value as Self)
    }

    fn into_jni<'local>(self, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        Ok(self as <Self as JavaType>::JniType<'local>)  // identical types
    }

    fn from_jvalue<'local>(jvalue: JValueOwned<'local>, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        match jvalue {
            JValueOwned::Short(short) => Ok(short),
            _ => Err(Some(Exception { class: "java/lang/ClassCastException".to_string(), msg: format!("{} cannot be cast to {}", jvalue.type_name(), <Self as JavaType>::QUALIFIED_NAME()) }))
        }
    }
}

/// Java short = rust u16 (short interpreted unsigned)
impl JavaType for u16 {
    type JniType<'local> = jshort;
    type ArrayType<'local> = JShortArray<'local>;

    fn QUALIFIED_NAME() -> &'static str { "short" }

    fn JVM_PARAM_SIGNATURE() -> &'static str { "S" }

    fn EXCEPTION_NULL<'local>() -> Self::JniType<'local> { 0 }

    fn from_jni<'local>(jni_value: Self::JniType<'local>, _env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        Ok(jni_value as Self)
    }

    fn into_jni<'local>(self, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        Ok(self as <Self as JavaType>::JniType<'local>)  // identical types
    }

    fn from_jvalue<'local>(jvalue: JValueOwned<'local>, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        match jvalue {
            JValueOwned::Short(short) => Ok(short),
            _ => Err(Some(Exception { class: "java/lang/ClassCastException".to_string(), msg: format!("{} cannot be cast to {}", jvalue.type_name(), <Self as JavaType>::QUALIFIED_NAME()) }))
        }
    }
}

/// Java int = rust i32
impl JavaType for i32 {
    type JniType<'local> = jint;
    type ArrayType<'local> = JIntArray<'local>;

    fn QUALIFIED_NAME() -> &'static str { "int" }

    fn JVM_PARAM_SIGNATURE() -> &'static str { "I" }

    fn EXCEPTION_NULL<'local>() -> Self::JniType<'local> { 0 }

    fn from_jni<'local>(jni_value: Self::JniType<'local>, _env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        Ok(jni_value as Self)
    }

    fn into_jni<'local>(self, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        Ok(self as <Self as JavaType>::JniType<'local>)  // identical types
    }

    fn from_jvalue<'local>(jvalue: JValueOwned<'local>, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        match jvalue {
            JValueOwned::Int(int) => Ok(int),
            _ => Err(Some(Exception { class: "java/lang/ClassCastException".to_string(), msg: format!("{} cannot be cast to {}", jvalue.type_name(), <Self as JavaType>::QUALIFIED_NAME()) }))
        }
    }
}

/// Java int = rust u32 (int interpreted unsigned)
impl JavaType for u32 {
    type JniType<'local> = jint;
    type ArrayType<'local> = JIntArray<'local>;

    fn QUALIFIED_NAME() -> &'static str { "int" }

    fn JVM_PARAM_SIGNATURE() -> &'static str { "I" }

    fn EXCEPTION_NULL<'local>() -> Self::JniType<'local> { 0 }

    fn from_jni<'local>(jni_value: Self::JniType<'local>, _env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        Ok(jni_value as Self)
    }

    fn into_jni<'local>(self, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        Ok(self as <Self as JavaType>::JniType<'local>)  // identical types
    }

    fn from_jvalue<'local>(jvalue: JValueOwned<'local>, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        match jvalue {
            JValueOwned::Int(int) => Ok(int),
            _ => Err(Some(Exception { class: "java/lang/ClassCastException".to_string(), msg: format!("{} cannot be cast to {}", jvalue.type_name(), <Self as JavaType>::QUALIFIED_NAME()) }))
        }
    }
}

/// Java long = rust i64
impl JavaType for i64 {
    type JniType<'local> = jlong;
    type ArrayType<'local> = JLongArray<'local>;

    fn QUALIFIED_NAME() -> &'static str { "long" }

    fn JVM_PARAM_SIGNATURE() -> &'static str { "J" }

    fn EXCEPTION_NULL<'local>() -> Self::JniType<'local> { 0 }

    fn from_jni<'local>(jni_value: Self::JniType<'local>, _env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        Ok(jni_value as Self)
    }

    fn into_jni<'local>(self, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        Ok(self as <Self as JavaType>::JniType<'local>)  // identical types
    }

    fn from_jvalue<'local>(jvalue: JValueOwned<'local>, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        match jvalue {
            JValueOwned::Long(long) => Ok(long),
            _ => Err(Some(Exception { class: "java/lang/ClassCastException".to_string(), msg: format!("{} cannot be cast to {}", jvalue.type_name(), <Self as JavaType>::QUALIFIED_NAME()) }))
        }
    }
}

/// Java long = rust u64 (long interpreted unsigned)
impl JavaType for u64 {
    type JniType<'local> = jlong;
    type ArrayType<'local> = JLongArray<'local>;

    fn QUALIFIED_NAME() -> &'static str { "long" }

    fn JVM_PARAM_SIGNATURE() -> &'static str { "J" }

    fn EXCEPTION_NULL<'local>() -> Self::JniType<'local> { 0 }

    fn from_jni<'local>(jni_value: Self::JniType<'local>, _env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        Ok(jni_value as Self)
    }

    fn into_jni<'local>(self, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        Ok(self as <Self as JavaType>::JniType<'local>)  // identical types
    }

    fn from_jvalue<'local>(jvalue: JValueOwned<'local>, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        match jvalue {
            JValueOwned::Long(long) => Ok(long),
            _ => Err(Some(Exception { class: "java/lang/ClassCastException".to_string(), msg: format!("{} cannot be cast to {}", jvalue.type_name(), <Self as JavaType>::QUALIFIED_NAME()) }))
        }
    }
}

/// Java float = rust f32
impl JavaType for f32 {
    type JniType<'local> = jfloat;
    type ArrayType<'local> = JFloatArray<'local>;

    fn QUALIFIED_NAME() -> &'static str { "float" }

    fn JVM_PARAM_SIGNATURE() -> &'static str { "F" }

    fn EXCEPTION_NULL<'local>() -> Self::JniType<'local> { 0.0 }

    fn from_jni<'local>(jni_value: Self::JniType<'local>, _env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        Ok(jni_value as Self)
    }

    fn into_jni<'local>(self, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        Ok(self as <Self as JavaType>::JniType<'local>)  // identical types
    }

    fn from_jvalue<'local>(jvalue: JValueOwned<'local>, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        match jvalue {
            JValueOwned::Float(float) => Ok(float),
            _ => Err(Some(Exception { class: "java/lang/ClassCastException".to_string(), msg: format!("{} cannot be cast to {}", jvalue.type_name(), <Self as JavaType>::QUALIFIED_NAME()) }))
        }
    }
}

/// Java double = rust f64
impl JavaType for f64 {
    type JniType<'local> = jdouble;
    type ArrayType<'local> = JDoubleArray<'local>;

    fn QUALIFIED_NAME() -> &'static str { "double" }

    fn JVM_PARAM_SIGNATURE() -> &'static str { "D" }

    fn EXCEPTION_NULL<'local>() -> Self::JniType<'local> { 0.0 }

    fn from_jni<'local>(jni_value: Self::JniType<'local>, _env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        Ok(jni_value as Self)
    }

    fn into_jni<'local>(self, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        Ok(self as <Self as JavaType>::JniType<'local>)  // identical types
    }

    fn from_jvalue<'local>(jvalue: JValueOwned<'local>, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        match jvalue {
            JValueOwned::Double(double) => Ok(double),
            _ => Err(Some(Exception { class: "java/lang/ClassCastException".to_string(), msg: format!("{} cannot be cast to {}", jvalue.type_name(), <Self as JavaType>::QUALIFIED_NAME()) }))
        }
    }
}

/// Java char = rust [`JavaChar`]
impl JavaType for JavaChar {
    type JniType<'local> = jchar;
    type ArrayType<'local> = JCharArray<'local>;

    fn QUALIFIED_NAME() -> &'static str { "char" }

    fn JVM_PARAM_SIGNATURE() -> &'static str { "C" }

    fn EXCEPTION_NULL<'local>() -> Self::JniType<'local> { 0 }

    fn from_jni<'local>(jni_value: Self::JniType<'local>, _env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        Ok(JavaChar(jni_value))
    }

    fn into_jni<'local>(self, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        Ok(self.0 as <Self as JavaType>::JniType<'local>)  // identical types
    }

    fn from_jvalue<'local>(jvalue: JValueOwned<'local>, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        match jvalue {
            JValueOwned::Char(char) => Ok(char),
            _ => Err(Some(Exception { class: "java/lang/ClassCastException".to_string(), msg: format!("{} cannot be cast to {}", jvalue.type_name(), <Self as JavaType>::QUALIFIED_NAME()) }))
        }
    }
}

/// Java String = rust String
impl JavaType for String {
    type JniType<'local> = JString<'local>;
    type ArrayType<'local> = JObjectArray<'local>;

    fn QUALIFIED_NAME() -> &'static str { "java.lang.String" }

    fn JVM_PARAM_SIGNATURE() -> &'static str { "Ljava/lang/String;" }

    fn EXCEPTION_NULL<'local>() -> Self::JniType<'local> { JString::from(JObject::null()) }

    fn from_jni<'local>(jni_value: Self::JniType<'local>, env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        if jni_value.is_null() {
            Err(Some(Exception { class: "java/lang/NullPointerException".to_string(), msg: format!("expected {}", <Self as JavaType>::QUALIFIED_NAME()) }))
        } else {
            env.get_string(&jni_value)
                .map(JavaStr::into)
                .map_err(map_jni_error)
        }
    }

    fn into_jni<'local>(self, env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        env.new_string(self)
            .map_err(map_jni_error)
    }

    fn from_jvalue<'local>(jvalue: JValueOwned<'local>, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        match jvalue {
            JValueOwned::Object(obj) => Ok(JString::from(obj)),
            _ => Err(Some(Exception { class: "java/lang/ClassCastException".to_string(), msg: format!("{} cannot be cast to {}", jvalue.type_name(), <Self as JavaType>::QUALIFIED_NAME()) }))
        }
    }
}

/// Java array = rust [`Box<[T]>`]
///
/// e.g. byte[] = `Box<[u8]>`, String[] = `Box<[String]>`
impl<T: JavaType> JavaType for Box<[T]> {
    type JniType<'local> = T::ArrayType<'local>;
    type ArrayType<'local> = JObjectArray<'local>;

    fn QUALIFIED_NAME() -> &'static str {
        static NAME: OnceLock<&'static str> = OnceLock::new();

        NAME.get_or_init(|| format!("{}[]", T::QUALIFIED_NAME()).leak())
    }

    fn JVM_PARAM_SIGNATURE() -> &'static str {
        static SIGNATURE: OnceLock<&'static str> = OnceLock::new();

        SIGNATURE.get_or_init(|| format!("[{}", T::JVM_PARAM_SIGNATURE()).leak())
    }

    fn EXCEPTION_NULL<'local>() -> Self::JniType<'local> {
        Self::JniType::EXCEPTION_NULL()
    }

    fn from_jni<'local>(jni_value: Self::JniType<'local>, env: &mut JNIEnv<'local>) -> Result<Self, Option<Exception>> {
        Self::JniType::from_jni(jni_value, env)
    }

    fn into_jni<'local>(self, env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        Self::JniType::into_jni(self, env)
    }

    fn from_jvalue<'local>(jvalue: JValueOwned<'local>, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        match jvalue {
            JValueOwned::Object(obj) => Ok(Self::JniType::from(obj)),
            _ => Err(Some(Exception { class: "java/lang/ClassCastException".to_string(), msg: format!("{} cannot be cast to {}", jvalue.type_name(), <Self as JavaType>::QUALIFIED_NAME()) }))
        }
    }
}

/// Types that may be used in FFI function returns
///
/// Superset of [`JavaType`] and `()` (void)
pub trait JavaReturn: Sized {
    /// Jni equivalent to this type; Used as type in FFI functions
    type JniType<'local>;

    /// Fully qualified java name of this type, such as "java.lang.Object"
    fn QUALIFIED_NAME() -> &'static str;

    /// JVM "internal" type signature, such as "Ljava/lang/Object;"
    fn JVM_PARAM_SIGNATURE() -> &'static str;

    /// 'Null' value to return to JNI in the event of exceptions. For objects this is a null pointer, for numerical types it is zero, for booleans it is false
    fn EXCEPTION_NULL<'local>() -> Self::JniType<'local>;

    /// Convert from rust type to JNI type
    fn into_jni<'local>(self, env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>>;

    // No from_jni or from_jvalue as these types are never used as function parameters or fields
}

/// Java void = rust ()
///
/// Permits 'void' functions to omit a return type and implicitly return ()
impl JavaReturn for () {
    type JniType<'local> = ();

    fn QUALIFIED_NAME() -> &'static str { "void" }

    fn JVM_PARAM_SIGNATURE() -> &'static str { "V" }

    fn EXCEPTION_NULL<'local>() -> Self::JniType<'local> { () }

    fn into_jni<'local>(self, _env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        Ok(self)
    }
}

/// All JavaTypes are valid return types
impl<T: JavaType> JavaReturn for T {
    type JniType<'local> = T::JniType<'local>;

    fn QUALIFIED_NAME() -> &'static str { T::QUALIFIED_NAME() }

    fn JVM_PARAM_SIGNATURE() -> &'static str { T::JVM_PARAM_SIGNATURE() }

    fn EXCEPTION_NULL<'local>() -> Self::JniType<'local> {
        <T as JavaType>::EXCEPTION_NULL()
    }

    fn into_jni<'local>(self, env: &mut JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<Exception>> {
        <T as JavaType>::into_jni(self, env)
    }
}