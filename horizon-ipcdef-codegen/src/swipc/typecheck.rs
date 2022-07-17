use crate::swipc::diagnostics;
use crate::swipc::diagnostics::{DiagnosticErrorExt, DiagnosticExt, DiagnosticResultExt, Span};
use crate::swipc::model::{
    Bitflags, BitflagsArm, Command, Enum, EnumArm, IntType, Interface, IpcFileItem,
    NamespacedIdent, Struct, StructField, StructuralType, TypeWithName, TypecheckContext, Value,
};
use arcstr::ArcStr;
use codespan_reporting::diagnostic::Diagnostic;
use convert_case::{Case, Casing};
use diagnostics::Result;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::sync::Arc;

impl Value {
    pub fn typecheck(&self, context: &TypecheckContext) -> Result<()> {
        match self {
            Value::ClientProcessId
            | Value::InHandle(_)
            | Value::OutHandle(_)
            | Value::InBuffer(_, _)
            | Value::OutBuffer(_, _) => Ok(()),
            Value::In(t) | Value::Out(t) | Value::InArray(t, _) | Value::OutArray(t, _) => {
                t.typecheck_resolve(context).map(|_| ())
            }
            Value::InObject(obj, location) => context.resolve_interface(obj, location).map(|_| ()),
            Value::OutObject(obj, location) => obj
                .as_ref()
                .map(|obj| context.resolve_interface(obj, location).map(|_| ()))
                .unwrap_or(Ok(())),
        }
    }
}

impl TypeWithName {
    pub fn resolve_and_typecheck(&self, context: &TypecheckContext) -> Result<StructuralType> {
        Ok(match self {
            TypeWithName::TypeAlias(t) => t.referenced_type.typecheck_resolve(context)?,
            TypeWithName::StructDef(s) => {
                s.typecheck(context)?;
                StructuralType::Struct(s.clone())
            }
            TypeWithName::EnumDef(e) => {
                e.typecheck(context)?;
                StructuralType::Enum(e.clone())
            }
            TypeWithName::BitflagsDef(b) => {
                b.typecheck(context)?;
                StructuralType::Bitflags(b.clone())
            }
        })
    }
}

impl StructField {
    pub fn typecheck(&self, context: &TypecheckContext) -> Result<()> {
        match self.ty.typecheck_resolve(context) {
            Ok(t) => {
                if !t.is_sized() {
                    return Err(vec![Diagnostic::error()
                        .with_message(format!("Use of unsized type in field `{}`", self.name))
                        .with_primary_label(self.location)]);
                }

                Ok(())
            }
            Err(e) => Err(e.with_context(self.location, || format!("In field `{}`", self.name))),
        }
    }
}

impl Struct {
    pub fn typecheck(&self, context: &TypecheckContext) -> Result<()> {
        let mut res = Ok(());

        let mut fields = BTreeMap::new();

        for field in self.fields.iter() {
            match fields.entry(&field.name) {
                Entry::Vacant(v) => {
                    v.insert(field);
                }
                Entry::Occupied(o) => res.push(
                    Diagnostic::error()
                        .with_message(format!("Duplicate struct field `{}`", field.name,))
                        .with_primary_label(field.location)
                        .with_secondary_label(o.get().location, "Previously defined here"),
                ),
            }

            res.extend_result(
                field
                    .typecheck(context)
                    .with_context(self.location, || format!("In struct `{}`", self.name)),
            );
        }

        res
    }
}

impl EnumArm {
    pub fn typecheck(&self, _context: &TypecheckContext, base_type: IntType) -> Result<()> {
        if !base_type.fits_u64(self.value) {
            return Err(vec![Diagnostic::error()
                .with_message(format!(
                    "Value {} of enum arm `{}` does not fit into type {:?}",
                    self.value, self.name, base_type
                ))
                .with_primary_label(self.location)]);
        }
        Ok(())
    }
}

impl Enum {
    pub fn typecheck(&self, context: &TypecheckContext) -> Result<()> {
        let mut res = Ok(());

        let mut arm_values: BTreeMap<u64, &EnumArm> = BTreeMap::new();
        let mut arm_names: BTreeMap<&ArcStr, &EnumArm> = BTreeMap::new();

        for arm in self.arms.iter() {
            res.extend_result(
                arm.typecheck(context, self.base_type)
                    .with_context(self.location, || format!("In enum `{}`", self.name)),
            );

            match arm_values.entry(arm.value) {
                Entry::Vacant(e) => {
                    e.insert(arm);
                }
                Entry::Occupied(o) => res.push(
                    Diagnostic::error()
                        .with_message("Duplicate enum value".to_string())
                        .with_primary_label(arm.location)
                        .with_secondary_label(o.get().location, "Previously defined here"),
                ),
            }

            match arm_names.entry(&arm.name) {
                Entry::Vacant(e) => {
                    e.insert(arm);
                }
                Entry::Occupied(o) => res.push(
                    Diagnostic::error()
                        .with_message(format!("Duplicate enum arm named `{}`", arm.name,))
                        .with_primary_label(arm.location)
                        .with_secondary_label(o.get().location, "Previously defined here"),
                ),
            }
        }

        if arm_values.get(&0).is_none() {
            // this is used as default value
            res.push(
                Diagnostic::error()
                    .with_message(format!(
                        "Enum `{}` should have an arm with value 0",
                        self.name
                    ))
                    .with_primary_label(self.location),
            );
        }

        res
    }
}

impl BitflagsArm {
    pub fn typecheck(&self, _context: &TypecheckContext, base_type: IntType) -> Result<()> {
        if !base_type.fits_u64(self.value) {
            return Err(vec![Diagnostic::error()
                .with_message(format!(
                    "Value {} of bitflags arm `{}` does not fit into type {:?}",
                    self.value, self.name, base_type
                ))
                .with_labels(vec![self.location.primary_label()])]);
        }
        Ok(())
    }
}

impl Bitflags {
    pub fn typecheck(&self, context: &TypecheckContext) -> Result<()> {
        let mut res = Ok(());

        let mut arm_names: BTreeMap<&ArcStr, &BitflagsArm> = BTreeMap::new();

        for arm in self.arms.iter() {
            res.extend_result(
                arm.typecheck(context, self.base_type)
                    .with_context(self.location, || format!("In bitflags `{}`", self.name)),
            );

            match arm_names.entry(&arm.name) {
                Entry::Vacant(e) => {
                    e.insert(arm);
                }
                Entry::Occupied(o) => res.push(
                    Diagnostic::error()
                        .with_message(format!("Duplicate bitfield arm named `{}`", arm.name,))
                        .with_primary_label(arm.location)
                        .with_secondary_label(o.get().location, "Previously defined here"),
                ),
            }
        }

        res
    }
}

impl Command {
    pub fn typecheck(&self, context: &TypecheckContext) -> Result<()> {
        let mut res = Ok(());

        for (_, arg) in self.arguments.iter() {
            res.extend_result(
                arg.typecheck(context)
                    .with_context(self.location, || format!("In command `{}`", self.name)),
            );
        }

        res
    }
}

impl Interface {
    pub fn typecheck(&self, context: &TypecheckContext) -> Result<()> {
        let mut res = Ok(());

        let mut command_names = BTreeMap::new();
        let mut command_ids = BTreeMap::new();

        for command in self.commands.iter() {
            res.extend_result(
                command
                    .typecheck(context)
                    .with_context(self.location, || format!("In interface `{}`", self.name)),
            );

            match command_names.entry(&command.name) {
                Entry::Vacant(v) => {
                    v.insert(command);
                }
                Entry::Occupied(o) => res.push(
                    Diagnostic::error()
                        .with_message(format!("Duplicate command named `{}`", command.name))
                        .with_primary_label(command.location)
                        .with_secondary_label(o.get().location, "Previous definition here")
                        .with_secondary_label(
                            self.location,
                            format!("In interface `{}`", self.name),
                        ),
                ),
            }

            match command_ids.entry(command.id) {
                Entry::Vacant(v) => {
                    v.insert(command);
                }
                Entry::Occupied(o) => res.push(
                    Diagnostic::error()
                        .with_message(format!("Duplicate command with id `{}`", command.id))
                        .with_primary_label(command.location)
                        .with_secondary_label(o.get().location, "Previous definition here")
                        .with_secondary_label(
                            self.location,
                            format!("In interface `{}`", self.name),
                        ),
                ),
            }
        }

        res
    }
}

fn case_name(case: Case) -> &'static str {
    match case {
        Case::Camel => "camelCase",
        Case::Pascal => "PascalCase",
        Case::Snake => "snake_case",
        Case::ScreamingSnake => "SCREAMING_SNAKE_CASE",
        _ => todo!("{:?} case", case),
    }
}

fn uppercase_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn check_naming(name: &str, item_type: &str, case: Case, location: Span) -> Result<()> {
    let cased = name.to_case(case);
    if cased != name {
        Err(vec![Diagnostic::warning()
            .with_message(format!(
                "{} name `{}` should be in {}, like `{}`",
                uppercase_first_letter(item_type),
                name,
                case_name(case),
                cased,
            ))
            .with_primary_label(location)])
    } else {
        Ok(())
    }
}

fn check_namespaced_naming(
    name: &NamespacedIdent,
    item_type: &str,
    case: Case,
    location: Span,
) -> Result<()> {
    let ident_cased = name.ident().to_case(case);

    let namespace_cased = name
        .namespace()
        .iter()
        .map(|p| p.as_str().to_case(Case::Snake))
        .collect::<Vec<_>>();

    let ident_wrong = ident_cased.as_str() != name.ident().as_str();
    let namespace_wrong = !itertools::equal(namespace_cased.iter(), name.namespace().iter());

    if ident_wrong || namespace_wrong {
        let cased = NamespacedIdent::new(
            Arc::new(
                namespace_cased
                    .into_iter()
                    .map(|s| ArcStr::from(s))
                    .collect(),
            ),
            ArcStr::from(ident_cased),
        );

        let message = if namespace_wrong && ident_wrong {
            format!(
                "{} name `{}` should have namespaces in camel_case and the identifier in {}, like `{}`",
                uppercase_first_letter(item_type),
                name,
                case_name(case),
                cased,
            )
        } else if namespace_wrong {
            format!(
                "{} name `{}` should have namespaces in camel_case, like `{}`",
                uppercase_first_letter(item_type),
                name,
                cased,
            )
        } else {
            format!(
                "{} name `{}` should have the identifier in {}, like `{}`",
                uppercase_first_letter(item_type),
                name,
                case_name(case),
                cased,
            )
        };

        Err(vec![Diagnostic::warning()
            .with_message(message)
            .with_primary_label(location)])
    } else {
        Ok(())
    }
}

impl IpcFileItem {
    pub fn check_naming(&self) -> Result<()> {
        let mut res = Ok(());

        match self {
            IpcFileItem::TypeAlias(a) => {
                res.extend_result(check_namespaced_naming(
                    &a.name,
                    "type alias",
                    Case::Pascal,
                    a.location,
                ));
            }
            IpcFileItem::StructDef(s) => {
                res.extend_result(check_namespaced_naming(
                    &s.name,
                    "struct",
                    Case::Pascal,
                    s.location,
                ));

                for field in s.fields.iter() {
                    res.extend_result(check_naming(
                        &field.name,
                        "struct field",
                        Case::Snake,
                        field.location,
                    ));
                }
            }
            IpcFileItem::EnumDef(e) => {
                res.extend_result(check_namespaced_naming(
                    &e.name,
                    "enum",
                    Case::Pascal,
                    e.location,
                ));

                for arm in e.arms.iter() {
                    res.extend_result(check_naming(
                        &arm.name,
                        "enum arm",
                        Case::Pascal,
                        arm.location,
                    ));
                }
            }
            IpcFileItem::BitflagsDef(b) => {
                res.extend_result(check_namespaced_naming(
                    &b.name,
                    "bitflags",
                    Case::Pascal,
                    b.location,
                ));

                for arm in b.arms.iter() {
                    res.extend_result(check_naming(
                        &arm.name,
                        "bitflags arm",
                        Case::Pascal,
                        arm.location,
                    ));
                }
            }
            IpcFileItem::InterfaceDef(i) => {
                res.extend_result(check_namespaced_naming(
                    &i.name,
                    "interface",
                    Case::Pascal,
                    i.location,
                ));

                for command in i.commands.iter() {
                    res.extend_result(check_naming(
                        &command.name,
                        "command",
                        Case::Pascal,
                        command.location,
                    ));

                    for (name, _) in command.arguments.iter() {
                        if let Some(name) = name {
                            res.extend_result(check_naming(
                                name,
                                "argument",
                                Case::Snake,
                                command.location,
                            ));
                        }
                    }
                }
            }
        }

        res
    }
}
