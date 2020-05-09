use crate::casting;
use crate::common::CaseInsensitiveString;
use crate::instruction_generator::NamedRefParam;
use crate::linter::*;
use crate::variant::Variant;
use std::collections::{HashMap, VecDeque};

//
// Argument
//

#[derive(Clone, Debug, PartialEq)]
pub enum Argument {
    ByVal(Variant),
    ByRef(QualifiedName),
}

//
// Cast
//

trait Cast {
    fn cast(self, qualifier: TypeQualifier) -> Self;
}

impl Cast for Variant {
    fn cast(self, qualifier: TypeQualifier) -> Self {
        casting::cast(self, qualifier).unwrap()
    }
}

impl Cast for Argument {
    fn cast(self, qualifier: TypeQualifier) -> Self {
        match self {
            Self::ByRef(n) => Self::ByRef(n),
            Self::ByVal(v) => Self::ByVal(casting::cast(v, qualifier).unwrap()),
        }
    }
}

//
// NameMap
//

#[derive(Debug)]
struct NameMap<T: std::fmt::Debug + Sized + Cast>(
    HashMap<CaseInsensitiveString, HashMap<TypeQualifier, T>>,
);

impl<T: std::fmt::Debug + Sized + Cast> NameMap<T> {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, name: QualifiedName, value: T) {
        let (bare_name, qualifier) = name.consume();
        match self.0.get_mut(&bare_name) {
            Some(inner_map) => {
                inner_map.insert(qualifier, value.cast(qualifier));
            }
            None => {
                let mut inner_map: HashMap<TypeQualifier, T> = HashMap::new();
                inner_map.insert(qualifier, value.cast(qualifier));
                self.0.insert(bare_name, inner_map);
            }
        }
    }

    pub fn get(&self, name: &QualifiedName) -> Option<&T> {
        match self.0.get(name.bare_name()) {
            Some(inner_map) => inner_map.get(&name.qualifier()),
            None => None,
        }
    }

    pub fn get_mut(&mut self, name: &QualifiedName) -> Option<&mut T> {
        match self.0.get_mut(name.bare_name()) {
            Some(inner_map) => inner_map.get_mut(&name.qualifier()),
            None => None,
        }
    }
}

//
// ConstantMap
//

#[derive(Debug)]
struct ConstantMap(HashMap<CaseInsensitiveString, Variant>);

impl ConstantMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get(&self, name: &QualifiedName) -> Option<&Variant> {
        match self.0.get(name.bare_name()) {
            Some(v) => {
                if name.qualifier() == v.qualifier() {
                    Some(v)
                } else {
                    // trying to reference a constant with wrong type
                    panic!("Duplicate definition")
                }
            }
            None => None,
        }
    }

    pub fn insert(&mut self, name: QualifiedName, value: Variant) {
        match self.0.get(name.bare_name()) {
            Some(_) => panic!("Duplicate definition"),
            None => {
                let (bare_name, qualifier) = name.consume();
                self.0.insert(bare_name, value.cast(qualifier));
            }
        }
    }
}

type VariableMap = NameMap<Variant>;
type ArgumentMap = NameMap<Argument>;
type UnnamedArgs = VecDeque<Argument>;

// TODO convert to struct and use only one data structure for both named and unnamed arguments
type Args = (ArgumentMap, UnnamedArgs);

//
// RootContext
//

#[derive(Debug)]
pub struct RootContext {
    variables: VariableMap,
    constants: ConstantMap,
}

impl RootContext {
    pub fn new() -> Self {
        Self {
            variables: NameMap::new(),
            constants: ConstantMap::new(),
        }
    }

    pub fn get_constant(&self, name: &QualifiedName) -> Option<Variant> {
        self.constants.get(name).map(|x| x.clone())
    }

    pub fn get_r_value(&self, name: &QualifiedName) -> Option<Variant> {
        // local constant?
        match self.constants.get(name) {
            Some(v) => Some(v.clone()),
            None => {
                // variable?
                match self.variables.get(name) {
                    Some(v) => Some(v.clone()),
                    None => None,
                }
            }
        }
    }

    pub fn set_constant(&mut self, name: QualifiedName, value: Variant) {
        self.constants.insert(name, value);
    }

    pub fn create_parameter(&mut self, name: QualifiedName) -> Argument {
        match self.get_constant(&name) {
            Some(v) => Argument::ByVal(v),
            None => {
                match self.variables.get(&name) {
                    // ref pointing to var
                    Some(_) => Argument::ByRef(name),
                    None => {
                        // create the variable in this scope
                        // e.g. INPUT N
                        self.variables
                            .insert(name.clone(), Variant::default_variant(name.qualifier()));
                        Argument::ByRef(name)
                    }
                }
            }
        }
    }

    pub fn set_variable(&mut self, name: QualifiedName, value: Variant) {
        // Arguments do not exist at root level. Create/Update a variable.
        self.variables.insert(name, value);
    }
}

//
// ArgsContext (collecting arguments just before a function/sub call)
//

#[derive(Debug)]
pub struct ArgsContext {
    parent: Box<Context>,
    args: Args,
}

impl ArgsContext {
    pub fn push_back_unnamed_ref_parameter(&mut self, name: QualifiedName) {
        let arg = self.parent.create_parameter(name);
        self.args.1.push_back(arg);
    }

    pub fn push_back_unnamed_val_parameter(&mut self, value: Variant) {
        self.args.1.push_back(Argument::ByVal(value));
    }

    pub fn set_named_ref_parameter(&mut self, named_ref_param: &NamedRefParam) {
        let arg = self
            .parent
            .create_parameter(named_ref_param.argument_name.clone());
        self.insert_next_argument(&named_ref_param.parameter_name, arg);
    }

    pub fn set_named_val_parameter(&mut self, param_name: &QualifiedName, value: Variant) {
        self.insert_next_argument(param_name, Argument::ByVal(value));
    }

    fn insert_next_argument(&mut self, param_name: &QualifiedName, arg: Argument) {
        self.args.0.insert(param_name.clone(), arg);
    }
}

//
// SubContext (inside a function or sub)
//

#[derive(Debug)]
pub struct SubContext {
    parent: Box<Context>,
    variables: ArgumentMap,
    constants: ConstantMap,
    unnamed_args: UnnamedArgs,
}

impl SubContext {
    fn set_variable_parent(&mut self, name: QualifiedName, value: Variant) {
        self.parent.set_variable(name, value)
    }

    fn do_insert_variable(&mut self, name: QualifiedName, value: Variant) {
        self.variables.insert(name, Argument::ByVal(value));
    }

    fn get_argument_mut(&mut self, name: &QualifiedName) -> Option<&mut Argument> {
        self.variables.get_mut(name)
    }

    fn evaluate_argument(&self, arg: &Argument) -> Option<Variant> {
        match arg {
            Argument::ByVal(v) => Some(v.clone()),
            Argument::ByRef(n) => self.parent.get_r_value(n),
        }
    }

    fn get_variable(&self, name: &QualifiedName) -> Option<&Argument> {
        self.variables.get(name)
    }

    pub fn set_constant(&mut self, name: QualifiedName, value: Variant) {
        self.constants.insert(name, value);
    }

    pub fn pop_front_unnamed(&mut self) -> Variant {
        self.try_pop_front_unnamed().unwrap()
    }

    pub fn try_pop_front_unnamed(&mut self) -> Option<Variant> {
        match self.unnamed_args.pop_front() {
            Some(arg) => self.evaluate_argument(&arg),
            None => None,
        }
    }

    pub fn pop_front_unnamed_arg(&mut self) -> Option<Argument> {
        self.unnamed_args.pop_front()
    }

    pub fn set_value_to_popped_arg(&mut self, arg: &Argument, value: Variant) {
        match arg {
            Argument::ByVal(_) => panic!("Expected variable"),
            Argument::ByRef(n) => {
                let q = n.clone(); // clone to break duplicate borrow
                self.set_variable_parent(q, value)
            }
        }
    }

    pub fn create_parameter(&mut self, name: QualifiedName) -> Argument {
        match self.get_constant(&name) {
            Some(v) => Argument::ByVal(v),
            None => {
                // variable?
                match self.get_variable(&name) {
                    // ref pointing to var
                    Some(_) => Argument::ByRef(name),
                    None => {
                        // parent constant?
                        match self.parent.get_root().get_constant(&name) {
                            Some(v) => Argument::ByVal(v),
                            None => {
                                // create the variable in this scope
                                // e.g. INPUT N
                                self.do_insert_variable(
                                    name.clone(),
                                    Variant::default_variant(name.qualifier()),
                                );
                                Argument::ByRef(name)
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn set_variable(&mut self, name: QualifiedName, value: Variant) {
        // if a parameter exists, set it (might be a ref)
        match self.get_argument_mut(&name) {
            Some(a) => {
                match a {
                    Argument::ByVal(_old_value) => {
                        *a = Argument::ByVal(value.cast(name.qualifier()));
                    }
                    Argument::ByRef(n) => {
                        let q = n.clone(); // clone needed to break duplicate borrow
                        self.set_variable_parent(q, value);
                    }
                }
            }
            None => {
                // A parameter does not exist. Create/Update a variable.
                self.do_insert_variable(name, value);
            }
        }
    }

    pub fn get_constant(&self, name: &QualifiedName) -> Option<Variant> {
        self.constants.get(name).map(|x| x.clone())
    }

    pub fn get_r_value(&self, name: &QualifiedName) -> Option<Variant> {
        // local constant?
        match self.get_constant(name) {
            Some(v) => Some(v),
            None => {
                // variable?
                match self.get_variable(name) {
                    Some(v) => self.evaluate_argument(v),
                    None => {
                        // top-level constant?
                        self.parent.get_root().get_constant(name)
                    }
                }
            }
        }
    }
}

//
// Context
//

#[derive(Debug)]
pub enum Context {
    Root(RootContext),
    Sub(SubContext),
    Args(ArgsContext),
}

impl Context {
    pub fn new() -> Self {
        Self::Root(RootContext::new())
    }

    pub fn push_args_context(self) -> Self {
        Self::Args(ArgsContext {
            parent: Box::new(self),
            args: (NameMap::new(), VecDeque::new()),
        })
    }

    pub fn swap_args_with_sub_context(self) -> Self {
        match self {
            Self::Args(a) => Self::Sub(SubContext {
                parent: a.parent,
                variables: a.args.0,
                constants: ConstantMap::new(),
                unnamed_args: a.args.1,
            }),
            _ => panic!("Not in an args context"),
        }
    }

    pub fn pop(self) -> Self {
        match self {
            Self::Root(_) => panic!("Stack underflow"),
            Self::Sub(s) => *s.parent,
            Self::Args(_) => panic!("Did not finish args building"),
        }
    }

    pub fn set_constant(&mut self, name: QualifiedName, value: Variant) {
        match self {
            Self::Root(r) => r.set_constant(name, value),
            Self::Sub(s) => s.set_constant(name, value),
            _ => panic!("Not allowed in an arg context"),
        }
    }

    pub fn demand_args(&mut self) -> &mut ArgsContext {
        match self {
            Self::Args(a) => a,
            _ => panic!("Not in an args context"),
        }
    }

    pub fn demand_sub(&mut self) -> &mut SubContext {
        match self {
            Self::Sub(s) => s,
            _ => panic!("Not in a subprogram context"),
        }
    }

    pub fn get_root(&self) -> &RootContext {
        match self {
            Self::Root(r) => r,
            Self::Args(a) => a.parent.get_root(),
            Self::Sub(s) => s.parent.get_root(),
        }
    }

    pub fn create_parameter(&mut self, name: QualifiedName) -> Argument {
        match self {
            Self::Root(r) => r.create_parameter(name),
            Self::Sub(s) => s.create_parameter(name),
            Self::Args(a) => a.parent.create_parameter(name),
        }
    }

    pub fn set_variable(&mut self, name: QualifiedName, value: Variant) {
        match self {
            Self::Root(r) => r.set_variable(name, value),
            Self::Sub(s) => s.set_variable(name, value),
            Self::Args(a) => a.parent.set_variable(name, value),
        }
    }

    pub fn get_r_value(&self, name: &QualifiedName) -> Option<Variant> {
        match self {
            Self::Root(r) => r.get_r_value(name),
            Self::Args(a) => a.parent.get_r_value(name),
            Self::Sub(s) => s.get_r_value(name),
        }
    }
}
