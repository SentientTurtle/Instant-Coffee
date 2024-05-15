//! Utility functions

use jni::errors::{Error, Exception};
use jni::JNIEnv;
use jni::objects::{JObject, JString};

/// Maps JNI errors into Exceptions
///
/// Returns None for Error::JavaException; Signalling an exception has already been thrown
///
/// # Arguments
///
/// * `error`: JNI error
///
/// returns: Option<Exception>
pub fn map_jni_error(error: jni::errors::Error) -> Option<Exception> {
    match error {
        Error::JavaException => None,
        error => Some(Exception { class: "java/lang/RuntimeException".to_string(), msg: format!("JNI error: {}", error) }),    // Bad error; Generated code manually checks for NPEs/Cast exceptions to provide better errors in-context // TODO: Actually do that
    }
}

/// Retrieves classname for the given JObject
///
/// # Arguments
///
/// * `object`: JObject to lookup class name for
/// * `env`: JNI Env
///
/// returns: Result<String, Option<Exception>>
pub fn obj_classname<'local>(object: &JObject<'local>, env: &mut JNIEnv<'local>) -> Result<String, Option<Exception>> {
    let class = env.get_object_class(object)
        .map_err(map_jni_error)?;

    let class_name = env.call_method(class, "getName", "()Ljava/lang/String;", &[])
        .map_err(map_jni_error)?
        .l()
        .map_err(map_jni_error)?;

    env.get_string(&JString::from(class_name))
        .map_err(map_jni_error)
        .map(|string| string.into())
}
