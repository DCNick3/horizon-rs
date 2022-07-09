use crate::swipc::diagnostics;
use crate::swipc::diagnostics::{DiagnosticErrorExt, DiagnosticExt, DiagnosticResultExt};
use crate::swipc::model::{
    Bitflags, BitflagsArm, Command, Enum, EnumArm, IntType, Interface, Struct, StructField,
    TypeWithName, TypecheckContext, Value,
};
use arcstr::ArcStr;
use codespan_reporting::diagnostic::Diagnostic;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

impl Value {
    pub fn typecheck(&self, context: &TypecheckContext) -> diagnostics::Result<()> {
        match self {
            Value::ClientProcessId
            | Value::InHandle(_)
            | Value::OutHandle(_)
            | Value::InBuffer(_, _)
            | Value::OutBuffer(_, _) => Ok(()),
            Value::In(t) | Value::Out(t) | Value::InArray(t, _) | Value::OutArray(t, _) => {
                t.resolve(context).map(|_| ())
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
    pub fn typecheck(&self, context: &TypecheckContext) -> diagnostics::Result<()> {
        match self {
            TypeWithName::TypeAlias(t) => {
                t.referenced_type.resolve(context)?;
                Ok(())
            }
            TypeWithName::StructDef(s) => s.typecheck(context),
            TypeWithName::EnumDef(e) => e.typecheck(context),
            TypeWithName::BitflagsDef(b) => b.typecheck(context),
        }
    }
}

impl StructField {
    pub fn typecheck(&self, context: &TypecheckContext) -> diagnostics::Result<()> {
        match self.ty.resolve(context) {
            Ok(t) => {
                if !t.is_sized() {
                    return Err(vec![Diagnostic::error()
                        .with_message(format!("Use of unsized type in field `{}`", self.name))
                        .with_primary_label(self.location)]);
                }

                Ok(())
            }
            Err(e) => {
                Err(e.with_context(self.location, || format!("In field `{}`", self.name)))
            }
        }
    }
}

impl Struct {
    pub fn typecheck(&self, context: &TypecheckContext) -> diagnostics::Result<()> {
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
    pub fn typecheck(
        &self,
        _context: &TypecheckContext,
        base_type: IntType,
    ) -> diagnostics::Result<()> {
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
    pub fn typecheck(&self, context: &TypecheckContext) -> diagnostics::Result<()> {
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

        res
    }
}

impl BitflagsArm {
    pub fn typecheck(
        &self,
        _context: &TypecheckContext,
        base_type: IntType,
    ) -> diagnostics::Result<()> {
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
    pub fn typecheck(&self, context: &TypecheckContext) -> diagnostics::Result<()> {
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
    pub fn typecheck(&self, context: &TypecheckContext) -> diagnostics::Result<()> {
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
    pub fn typecheck(&self, context: &TypecheckContext) -> diagnostics::Result<()> {
        let mut res = Ok(());

        let mut command_names = BTreeMap::new();
        let mut command_ids = BTreeMap::new();

        for command in self.commands.iter() {
            res.extend_result(
                command
                    .typecheck(context)
                    .with_context(self.location, || format!("In command `{}`", self.name)),
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
