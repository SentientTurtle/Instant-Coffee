//! Java code generation

use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io;
use std::path::PathBuf;
use crate::JavaType;

/// Java field & method access modifier
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum JAccessModifier {
    Public,
    Protected,
    PackagePrivate,
    Private,
}

impl Display for JAccessModifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            JAccessModifier::Public => write!(f, "public"),
            JAccessModifier::Protected => write!(f, "protected"),
            JAccessModifier::PackagePrivate => write!(f, ""),
            JAccessModifier::Private => write!(f, "private"),
        }
    }
}

/// Java field descriptor
pub struct JField {
    /// Access modifier
    pub access: JAccessModifier,
    /// Java type of this field, as verbatim in Java source. Usually a JavaType::QUALIFIED_NAME()
    pub jtype: &'static str,
    /// Name of this field, as verbatim in Java source
    pub name: &'static str,
}

/// Java method descriptor
///
/// Currently only describes `native` methods
pub struct JMethod {
    /// True if this method is 'static'
    pub is_static: bool,
    /// Name of this method, as verbatim in Java source
    pub name: &'static str,
    /// Parameters of this method, as verbatim in Java source. Each entry is a tuple of (parameter name, parameter type)
    pub inputs: Vec<(&'static str, &'static str)>,
    /// Return type of this method, as verbatim in Java source
    pub output: &'static str,
}

impl JMethod {
    /// Write this method declaration's Java source to the specified io::Write
    pub fn write_method<W: io::Write>(&self, out: &mut W) -> io::Result<()> {
        if self.is_static {
            write!(out, "\tpublic static native {} {}(", self.output, self.name)?;
        } else {
            write!(out, "\tpublic native {} {}(", self.output, self.name)?;
        }
        let mut first = true;
        for (name, param_type) in &self.inputs {
            if first {
                first = false;
            } else {
                write!(out, ", ")?;
            }
            write!(out, "{} {}", param_type, name)?;
        }
        writeln!(out, ");")
    }
}

/// Java "tagged union" variant declaration
///
/// Unions/Enums-with-fields are implemented through sealed classes and polymorphism.
///
/// This struct represents one inner-class of a [`JClassDecl::EnumTaggedUnion`]
pub struct JUnionVariant {
    /// Classname of this variant, as verbatim in Java source.
    pub name: &'static str,
    /// Fields for this class
    pub fields: Vec<JField>,
}

/// Java class declaration
///
/// All classes are final
pub enum JClassDecl {
    /// Regular Java class
    Class {
        /// Classname, as verbatim in Java source
        name: &'static str,
        /// Fully qualified package, as verbatim in Java source
        package: &'static str,
        /// Fields for this class
        fields: Vec<JField>,
        /// Methods for this class
        methods: Vec<JMethod>,
    },
    /// Java enum; Equivalent to a field-less rust enum
    Enum {
        /// Classname, as verbatim in Java source
        name: &'static str,
        /// Fully qualified package, as verbatim in Java source
        package: &'static str,
        /// Enum variant names, as verbatim in Java source
        variants: Vec<&'static str>,
        /// Methods for this class
        methods: Vec<JMethod>,
    },
    /// Java 'tagged union'; A sealed class with a fixed set of direct subclasses, emulating rust enums with fields
    EnumTaggedUnion {
        /// Classname for the outer type, as verbatim in Java source
        name: &'static str,
        /// Fully qualified package, as verbatim in Java source
        package: &'static str,
        /// Enum variants; Inner subclasses
        variants: Vec<JUnionVariant>,
        /// Methods for the outer class
        methods: Vec<JMethod>,
    },
}

impl JClassDecl {
    /// Classname, as verbatim in Java source
    pub fn class_name(&self) -> &'static str {
        match self {
            JClassDecl::Class { name, .. } => name,
            JClassDecl::Enum { name, .. } => name,
            JClassDecl::EnumTaggedUnion { name, .. } => name
        }
    }

    /// Write this class declaration's Java source to the specified io::Write
    ///
    /// This must write to a .java file with the same name ([`Self::class_name()`]) as the class
    /// [`JModuleDecl::write_to_dir`] and [`JModuleDecl::write_jar`] perform this automatically
    pub fn write_class_file<W: io::Write>(&self, out: &mut W) -> io::Result<()> {
        match self {
            JClassDecl::Class { name, package, fields, methods } => {
                writeln!(out, "package {};\n", package)?;

                write!(out, "public final class {} {{", name)?;
                if fields.len() > 0 || methods.len() > 0 {
                    writeln!(out)?;
                }
                // Fields
                for field in fields {
                    writeln!(out, "\t{} {} {};", field.access, field.jtype, field.name)?;
                }
                if fields.len() > 0 {
                    writeln!(out)?;
                }

                // Constructor
                write!(out, "\tprivate {}(", name)?;
                for (idx, field) in fields.iter().enumerate() {
                    write!(out, "{} {}", field.jtype, field.name)?;
                    if idx != fields.len() - 1 {
                        write!(out, ", ")?;
                    }
                }
                if fields.len() > 0 {
                    writeln!(out, ") {{")?;
                    for field in fields {
                        writeln!(out, "\t\tthis.{} = {};", field.name, field.name)?;
                    }
                    writeln!(out, "\t}}")?;
                } else {
                    writeln!(out, ") {{}}")?;
                }

                if methods.len() > 0 {
                    writeln!(out)?;
                }
                // Methods
                for method in methods {
                    method.write_method(out)?
                }

                write!(out, "}}")?;
            }
            JClassDecl::Enum { name, package, variants, methods } => {
                writeln!(out, "package {};\n", package)?;

                write!(out, "public enum {} {{", name)?;

                if variants.len() > 0 {
                    writeln!(out)?;
                }
                let mut first = true;
                for variant in variants {
                    if first {
                        first = false;
                    } else {
                        writeln!(out, ",")?;
                    }
                    write!(out, "\t{}", variant)?;
                }
                if variants.len() > 0 {
                    writeln!(out, ";")?;
                }

                if methods.len() > 0 {
                    writeln!(out)?;
                }
                // Methods
                for method in methods {
                    method.write_method(out)?
                }

                write!(out, "}}")?;
            }
            JClassDecl::EnumTaggedUnion { name: enum_name, package, variants, methods } => {
                writeln!(out, "package {};\n", package)?;

                write!(out, "public abstract sealed class {} {{", enum_name)?;

                if variants.len() > 0 {
                    writeln!(out)?;
                }
                for variant in variants {
                    write!(out, "\tpublic static final class {} extends {} {{", variant.name, enum_name)?;

                    if variant.fields.len() > 0 {
                        writeln!(out)?;
                    }
                    // Fields
                    for field in &variant.fields {
                        writeln!(out, "\t\t{} {} {};", field.access, field.jtype, field.name)?;
                    }

                    if variant.fields.len() > 0 {
                        writeln!(out)?;
                    }

                    // Constructor
                    write!(out, "\t\tpublic {}(", variant.name)?;
                    for (idx, field) in variant.fields.iter().enumerate() {
                        write!(out, "{} {}", field.jtype, field.name)?;
                        if idx != variant.fields.len() - 1 {
                            write!(out, ", ")?;
                        }
                    }
                    if variant.fields.len() > 0 {
                        writeln!(out, ") {{")?;
                        for field in &variant.fields {
                            writeln!(out, "\t\t\tthis.{} = {};", field.name, field.name)?;
                        }
                        writeln!(out, "\t\t}}")?;
                    } else {
                        writeln!(out, ") {{}}")?;
                    }

                    writeln!(out, "\t}}")?;
                }

                if methods.len() > 0 {
                    writeln!(out)?;
                }
                // Methods
                for method in methods {
                    method.write_method(out)?
                }

                write!(out, "}}")?;
            }
        }

        Ok(())
    }
}

/// Struct representing an abstract Java package
///
/// (Currently) does not support module-info files
pub struct JModuleDecl {
    /// Module name, fully qualified, as verbatim in Java source
    pub name: &'static str,
    /// Classes in this module
    pub classes: Vec<JClassDecl>,
}

impl JModuleDecl {    // TODO: module-info.java generation
    /// Write this module to the specified directory
    ///
    /// If module name is fully qualified, package directory tree is generated
    ///
    /// Does not clear specified directory, but may overwrite files
    pub fn write_to_dir<T: AsRef<std::path::Path>>(&self, path: T) -> io::Result<()> {
        let mut package_path = PathBuf::from(path.as_ref());
        package_path.push(self.name.replace('.', "/"));

        std::fs::create_dir_all(&package_path)?;

        for class in &self.classes {
            let file_path = package_path.join(format!("{}.java", class.class_name()));
            class.write_class_file(&mut File::create(file_path)?)?;
        }

        Ok(())
    }

    /// Write this module to a jar file output stream
    ///
    /// If module name is fully qualified, package directory tree is generated
    #[cfg(feature = "codegen-jar")]
    pub fn write_jar<W: io::Write + io::Seek>(&self, out: &mut W) -> io::Result<()> {
        use zip::result::ZipError;
        use zip::write::SimpleFileOptions;

        let path = self.name.replace('.', "/");
        let mut writer = zip::ZipWriter::new(out);
        for class in &self.classes {
            writer.start_file(format!("{}/{}.java", path, class.class_name()), SimpleFileOptions::default()).unwrap();

            class.write_class_file(&mut writer)?;
        }

        writer.finish()
            .map_err(|e| match e {
                ZipError::Io(err) => err,
                e => io::Error::new(io::ErrorKind::Other, e)
            })
            .map(|_| ())
    }


    /// Write this module to a [`FFIJarBlob`]
    ///
    /// Generates an in-memory jar file, which may be passed through FFI
    #[cfg(feature = "codegen-ffi")]
    pub fn as_ffi_blob(&self) -> FFIJarBlob {   // TODO: Maybe remove FFI export
        let mut buffer = io::Cursor::new(Vec::new());
        self.write_jar(&mut buffer).expect("error writing jmodule to memory");

        FFIJarBlob::new(buffer.into_inner())
    }
}

/// FFI-safe byte array, enabling retrieval of java code from compiled binary
#[cfg(feature = "codegen-ffi")]
#[repr(C)]
pub struct FFIJarBlob {
    bytes: *mut u8,
    length: usize,
    capacity: usize,
}

#[cfg(feature = "codegen-ffi")]
impl FFIJarBlob {
    pub fn new(bytes: Vec<u8>) -> FFIJarBlob {
        let mut bytes = std::mem::ManuallyDrop::new(bytes);

        FFIJarBlob {
            bytes: bytes.as_mut_ptr(),
            length: bytes.len(),
            capacity: bytes.capacity(),
        }
    }

    pub unsafe fn into_vec(self) -> Vec<u8> {
        Vec::from_raw_parts(self.bytes, self.length, self.capacity)
    }
}

/// Trait for types that declare a Java class
///
/// Custom/Derive'd [`JavaType`] structs will implement this
pub trait JavaClass: Sized + JavaType {
    fn declaration() -> JClassDecl;
}
