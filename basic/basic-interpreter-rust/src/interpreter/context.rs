use super::{InterpreterError, Result};
use crate::casting::cast;
use crate::common::{CaseInsensitiveString, HasLocation, Location};
use crate::parser::*;
use crate::variant::Variant;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub enum Argument {
    ByVal(Variant),
    ByRef(QualifiedName),
}

// N = 1 (RootContext Variable N = 1)
// Hello N, 1
// -- Hello -> Intermediate of Root
//             N -> ref to Root.N
//             1 -> val
//             replace Intermediate with SubProgram of Root
//
// Hello 1, Fn(1)
// -- Hello -> Intermediate of Root
//             1 -> val
//             Fn -> Intermediate of Intermediate of Root
//                  1 -> val
//                  replace Intermediate of Intermediate of Root with Subprogram of Intermediate of Root
//             result -> val
//             replace Intermediate with Subprogram of Root

// 1. Constants are visible inside SUBs and maybe redefined once
// 2. Types of bare constant names are derived by the value of the expression
//    (e.g. CONST X = 1 is an integer while X = 1 is a float)
// 3. Variables are passed by ref to subs and functions
// 4. Assigning a function result can be done as a bare name
// 5. Accessing a constant of function typed but with the wrong type is an error
// 6. It is possible to have variable A% and A$ (this is not possible for
//    constants and function names)
// 7. Sub names are always bare (as they do not return a value)
//
// Use cases
// 1. LValue (e.g. X = ?, FOR X =)
//    Must not be constant
// 2. RValue (e.g. IF X, _ = X)
//    Constants, arguments, variables, all allowed.
// 3. Const LValue e.g. CONST X = 42
//    Allow redefine (but only once) within subprogram
//    Inherit in subprograms
//    Read only (reassign is error)
// 4. Const RValue e.g. CONST _ = X + 1 (where X is const)
//    It should complain for all non const values (i.e. it should complain for
//    function calls and names that are not const)
// 5. Ref Parameter (?)
//    INPUT N
//    Push to stack as reference.
//    If constant, push as value.
// 6. Val Parameter (?)
//    PRINT "hi"
//    Push to stack as variant
// 7. Get/Set function result

fn do_cast(value: Variant, qualifier: TypeQualifier, pos: Location) -> Result<Variant> {
    cast(value, qualifier).map_err(|e| InterpreterError::new_with_pos(e, pos))
}

pub type VariableMap = HashMap<CaseInsensitiveString, HashMap<TypeQualifier, Variant>>;
pub type ConstantMap = HashMap<CaseInsensitiveString, Variant>;
pub type ArgumentMap = HashMap<CaseInsensitiveString, HashMap<TypeQualifier, Argument>>;
pub type UnnamedArgs = VecDeque<Argument>;
pub type Args = (ArgumentMap, UnnamedArgs);
pub type FunctionResult = Option<Variant>;

#[derive(Debug)]
pub struct RootContext<T: TypeResolver> {
    resolver: Rc<RefCell<T>>,
    variables: VariableMap,
    constants: ConstantMap,
    function_result: Variant,
}

#[derive(Debug)]
pub struct ArgsContext<T: TypeResolver> {
    parent: Box<Context<T>>,
    args: Args,
}

#[derive(Debug)]
pub struct SubContext<T: TypeResolver> {
    parent: Box<Context<T>>,
    variables: ArgumentMap,
    constants: ConstantMap,
    unnamed_args: UnnamedArgs,
}

#[derive(Debug)]
pub enum Context<T: TypeResolver> {
    Root(RootContext<T>),
    Sub(SubContext<T>),
    Args(ArgsContext<T>),
}

trait CreateParameter {
    fn create_parameter(&mut self, name_node: &NameNode) -> Result<Argument>;
}

trait SetLValueQ {
    fn set_l_value_q(
        &mut self,
        bare_name: &CaseInsensitiveString,
        qualifier: TypeQualifier,
        pos: Location,
        value: Variant,
    ) -> Result<()>;
}

trait GetConstant {
    fn get_constant(
        &self,
        bare_name: &CaseInsensitiveString,
        opt_qualifier: Option<TypeQualifier>,
        pos: Location,
    ) -> Result<Option<&Variant>>;
}

impl GetConstant for ConstantMap {
    fn get_constant(
        &self,
        bare_name: &CaseInsensitiveString,
        opt_qualifier: Option<TypeQualifier>,
        pos: Location,
    ) -> Result<Option<&Variant>> {
        match self.get(bare_name) {
            Some(v) => {
                if opt_qualifier.is_none() || opt_qualifier.unwrap() == v.qualifier() {
                    Ok(Some(v))
                } else {
                    // trying to reference a constant with wrong type
                    Err(InterpreterError::new_with_pos("Duplicate definition", pos))
                }
            }
            None => Ok(None),
        }
    }
}

impl<T: TypeResolver> GetConstant for RootContext<T> {
    fn get_constant(
        &self,
        bare_name: &CaseInsensitiveString,
        opt_qualifier: Option<TypeQualifier>,
        pos: Location,
    ) -> Result<Option<&Variant>> {
        self.constants.get_constant(bare_name, opt_qualifier, pos)
    }
}

impl<T: TypeResolver> GetConstant for SubContext<T> {
    fn get_constant(
        &self,
        bare_name: &CaseInsensitiveString,
        opt_qualifier: Option<TypeQualifier>,
        pos: Location,
    ) -> Result<Option<&Variant>> {
        self.constants.get_constant(bare_name, opt_qualifier, pos)
    }
}

impl<T: TypeResolver> GetConstant for Context<T> {
    fn get_constant(
        &self,
        bare_name: &CaseInsensitiveString,
        opt_qualifier: Option<TypeQualifier>,
        pos: Location,
    ) -> Result<Option<&Variant>> {
        match self {
            Self::Root(r) => r.get_constant(bare_name, opt_qualifier, pos),
            Self::Args(a) => a.parent.get_constant(bare_name, opt_qualifier, pos),
            Self::Sub(s) => s.get_constant(bare_name, opt_qualifier, pos),
        }
    }
}

trait GetParentConstant {
    fn get_parent_constant(
        &self,
        bare_name: &CaseInsensitiveString,
        opt_qualifier: Option<TypeQualifier>,
        pos: Location,
    ) -> Result<Option<Variant>>;
}

impl<T: TypeResolver> GetParentConstant for RootContext<T> {
    fn get_parent_constant(
        &self,
        bare_name: &CaseInsensitiveString,
        opt_qualifier: Option<TypeQualifier>,
        pos: Location,
    ) -> Result<Option<Variant>> {
        Ok(None)
    }
}

impl<T: TypeResolver> GetParentConstant for ArgsContext<T> {
    fn get_parent_constant(
        &self,
        bare_name: &CaseInsensitiveString,
        opt_qualifier: Option<TypeQualifier>,
        pos: Location,
    ) -> Result<Option<Variant>> {
        match self.parent.get_constant(bare_name, opt_qualifier, pos)? {
            Some(v) => Ok(Some(v.clone())),
            None => self
                .parent
                .get_parent_constant(bare_name, opt_qualifier, pos),
        }
    }
}

impl<T: TypeResolver> GetParentConstant for SubContext<T> {
    fn get_parent_constant(
        &self,
        bare_name: &CaseInsensitiveString,
        opt_qualifier: Option<TypeQualifier>,
        pos: Location,
    ) -> Result<Option<Variant>> {
        match self.parent.get_constant(bare_name, opt_qualifier, pos)? {
            Some(v) => Ok(Some(v.clone())),
            None => self
                .parent
                .get_parent_constant(bare_name, opt_qualifier, pos),
        }
    }
}

impl<T: TypeResolver> GetParentConstant for Context<T> {
    fn get_parent_constant(
        &self,
        bare_name: &CaseInsensitiveString,
        opt_qualifier: Option<TypeQualifier>,
        pos: Location,
    ) -> Result<Option<Variant>> {
        match self {
            Self::Root(r) => r.get_parent_constant(bare_name, opt_qualifier, pos),
            Self::Args(a) => a.get_parent_constant(bare_name, opt_qualifier, pos),
            Self::Sub(s) => s.get_parent_constant(bare_name, opt_qualifier, pos),
        }
    }
}

trait GetRValueQualified {
    fn get_r_value_q(
        &self,
        bare_name: &CaseInsensitiveString,
        opt_qualifier: Option<TypeQualifier>,
        pos: Location,
    ) -> Result<Option<Variant>>;
}

impl<T: TypeResolver> GetRValueQualified for RootContext<T> {
    fn get_r_value_q(
        &self,
        bare_name: &CaseInsensitiveString,
        opt_qualifier: Option<TypeQualifier>,
        pos: Location,
    ) -> Result<Option<Variant>> {
        // local constant?
        match self.constants.get_constant(bare_name, opt_qualifier, pos)? {
            Some(v) => Ok(Some(v.clone())),
            None => {
                let qualifier = match opt_qualifier {
                    Some(q) => q,
                    None => self.resolver.resolve(bare_name),
                };
                // variable?
                match self.get_variable(bare_name, qualifier) {
                    Some(v) => Ok(Some(v.clone())),
                    None => Ok(None),
                }
            }
        }
    }
}

impl<T: TypeResolver> GetRValueQualified for ArgsContext<T> {
    fn get_r_value_q(
        &self,
        bare_name: &CaseInsensitiveString,
        opt_qualifier: Option<TypeQualifier>,
        pos: Location,
    ) -> Result<Option<Variant>> {
        self.parent.get_r_value_q(bare_name, opt_qualifier, pos)
    }
}

impl<T: TypeResolver> GetRValueQualified for SubContext<T> {
    fn get_r_value_q(
        &self,
        bare_name: &CaseInsensitiveString,
        opt_qualifier: Option<TypeQualifier>,
        pos: Location,
    ) -> Result<Option<Variant>> {
        // local constant?
        match self.get_constant(bare_name, opt_qualifier, pos)? {
            Some(v) => Ok(Some(v.clone())),
            None => {
                // argument?
                let qualifier = match opt_qualifier {
                    Some(q) => q,
                    None => self.resolve(bare_name),
                };
                // variable?
                match self.get_variable(bare_name, qualifier) {
                    Some(v) => self.evaluate_argument(v, pos),
                    None => {
                        // parent constant?
                        self.get_parent_constant(bare_name, opt_qualifier, pos)
                    }
                }
            }
        }
    }
}

impl<T: TypeResolver> GetRValueQualified for Context<T> {
    fn get_r_value_q(
        &self,
        bare_name: &CaseInsensitiveString,
        opt_qualifier: Option<TypeQualifier>,
        pos: Location,
    ) -> Result<Option<Variant>> {
        match self {
            Self::Root(r) => r.get_r_value_q(bare_name, opt_qualifier, pos),
            Self::Args(a) => a.get_r_value_q(bare_name, opt_qualifier, pos),
            Self::Sub(s) => s.get_r_value_q(bare_name, opt_qualifier, pos),
        }
    }
}

//
// RootContext
//

impl<T: TypeResolver> RootContext<T> {
    pub fn new(resolver: Rc<RefCell<T>>) -> Self {
        Self {
            resolver,
            variables: HashMap::new(),
            constants: HashMap::new(),
            function_result: Variant::VInteger(0),
        }
    }

    //
    // LValue (e.g. X = ?, FOR X = ?)
    //

    fn do_insert_variable(
        &mut self,
        bare_name: CaseInsensitiveString,
        qualifier: TypeQualifier,
        value: Variant,
    ) {
        match self.variables.get_mut(&bare_name) {
            Some(inner_map) => {
                inner_map.insert(qualifier, value);
            }
            None => {
                let mut inner_map: HashMap<TypeQualifier, Variant> = HashMap::new();
                inner_map.insert(qualifier, value);
                self.variables.insert(bare_name, inner_map);
            }
        }
    }

    fn constant_exists_no_recursion<U: NameTrait>(&self, name_node: &U) -> bool {
        self.constants.contains_key(name_node.bare_name())
    }

    //
    // RValue
    //

    pub fn get_r_value(&self, name_node: &NameNode) -> Result<Option<Variant>> {
        let bare_name = name_node.bare_name();
        let opt_qualifier = name_node.opt_qualifier();
        let pos = name_node.location();
        self.get_r_value_q(bare_name, opt_qualifier, pos)
    }

    fn get_variable(
        &self,
        bare_name: &CaseInsensitiveString,
        qualifier: TypeQualifier,
    ) -> Option<&Variant> {
        match self.variables.get(bare_name) {
            Some(inner_map) => inner_map.get(&qualifier),
            None => None,
        }
    }

    //
    // Const LValue
    //

    pub fn set_const_l_value(&mut self, name_node: &NameNode, value: Variant) -> Result<()> {
        let pos = name_node.location();
        // subtle difference, bare name constants get their type from the value
        let bare_name: &CaseInsensitiveString;
        let casted: Variant;
        match name_node.as_ref() {
            Name::Bare(b) => {
                bare_name = b;
                casted = value;
            }
            Name::Qualified(q) => {
                bare_name = q.bare_name();
                let qualifier = q.qualifier();
                casted = do_cast(value, qualifier, pos)?;
            }
        }
        // if a local constant or parameter or variable already exists throw an error
        if self.constant_exists_no_recursion(name_node) || self.variables.contains_key(bare_name) {
            return Err(InterpreterError::new_with_pos("Duplicate definition", pos));
        }
        // set it
        self.constants.insert(bare_name.clone(), casted);
        Ok(())
    }

    //
    // Const RValue
    //

    // TODO why is this not used?

    pub fn get_const_r_value(&self, name_node: &NameNode) -> Result<Variant> {
        let bare_name = name_node.bare_name();
        let opt_qualifier = name_node.opt_qualifier();
        let pos = name_node.location();
        if self.variables.contains_key(bare_name) {
            return Err(InterpreterError::new_with_pos("Invalid constant", pos));
        }

        match self.get_constant(bare_name, opt_qualifier, pos)? {
            Some(v) => Ok(v.clone()),
            None => Err(InterpreterError::new_with_pos(
                "Invalid constant (undef variable)",
                pos,
            )),
        }
    }
}

//
// RootContext traits
//

impl<T: TypeResolver> TypeResolver for RootContext<T> {
    fn resolve<U: NameTrait>(&self, n: &U) -> TypeQualifier {
        self.resolver.resolve(n)
    }
}

impl<T: TypeResolver> CreateParameter for RootContext<T> {
    fn create_parameter(&mut self, name_node: &NameNode) -> Result<Argument> {
        let bare_name = name_node.bare_name();
        let opt_qualifier = name_node.opt_qualifier();
        let pos = name_node.location();

        match self.get_constant(bare_name, opt_qualifier, pos)? {
            Some(v) => Ok(Argument::ByVal(v.clone())),
            None => {
                let qualifier = match opt_qualifier {
                    Some(q) => q,
                    None => self.resolve(bare_name),
                };
                match self.get_variable(bare_name, qualifier) {
                    // ref pointing to var
                    Some(_) => Ok(Argument::ByRef(QualifiedName::new(
                        bare_name.clone(),
                        qualifier,
                    ))),
                    None => {
                        // create the variable in this scope
                        // e.g. INPUT N
                        self.do_insert_variable(
                            bare_name.clone(),
                            qualifier,
                            Variant::default_variant(qualifier),
                        );
                        Ok(Argument::ByRef(QualifiedName::new(
                            bare_name.clone(),
                            qualifier,
                        )))
                    }
                }
            }
        }
    }
}

impl<T: TypeResolver> SetLValueQ for RootContext<T> {
    fn set_l_value_q(
        &mut self,
        bare_name: &CaseInsensitiveString,
        qualifier: TypeQualifier,
        pos: Location,
        value: Variant,
    ) -> Result<()> {
        // if a constant exists, throw error
        if self.constant_exists_no_recursion(bare_name) {
            return Err(InterpreterError::new_with_pos("Duplicate definition", pos));
        }
        // Arguments do not exist at root level. Create/Update a variable.
        let casted = do_cast(value, qualifier, pos)?;
        self.do_insert_variable(bare_name.clone(), qualifier, casted);
        Ok(())
    }
}

//
// ArgsContext
//

impl<T: TypeResolver> ArgsContext<T> {
    pub fn push_back_unnamed_ref_parameter(&mut self, name_node: &NameNode) -> Result<()> {
        let arg = self.create_parameter(name_node)?;
        self.args.1.push_back(arg);
        Ok(())
    }

    pub fn push_back_unnamed_val_parameter(&mut self, value: Variant) {
        self.args.1.push_back(Argument::ByVal(value));
    }

    pub fn set_named_ref_parameter(
        &mut self,
        param_name: &QualifiedName,
        name_node: &NameNode,
    ) -> Result<()> {
        let arg = self.create_parameter(name_node)?;
        self.insert_next_argument(param_name, arg);
        Ok(())
    }

    pub fn set_named_val_parameter(&mut self, param_name: &QualifiedName, value: Variant) {
        self.insert_next_argument(param_name, Argument::ByVal(value));
    }

    fn insert_next_argument(&mut self, param_name: &QualifiedName, arg: Argument) {
        match self.args.0.get_mut(param_name.bare_name()) {
            Some(inner_map) => {
                inner_map.insert(param_name.qualifier(), arg);
            }
            None => {
                let mut inner_map: HashMap<TypeQualifier, Argument> = HashMap::new();
                inner_map.insert(param_name.qualifier(), arg);
                self.args
                    .0
                    .insert(param_name.bare_name().clone(), inner_map);
            }
        }
    }
}

//
// ArgsContext traits
//

impl<T: TypeResolver> TypeResolver for ArgsContext<T> {
    fn resolve<U: NameTrait>(&self, n: &U) -> TypeQualifier {
        self.parent.resolve(n)
    }
}

impl<T: TypeResolver> CreateParameter for ArgsContext<T> {
    fn create_parameter(&mut self, name_node: &NameNode) -> Result<Argument> {
        self.parent.create_parameter(name_node)
    }
}

impl<T: TypeResolver> SetLValueQ for ArgsContext<T> {
    fn set_l_value_q(
        &mut self,
        bare_name: &CaseInsensitiveString,
        qualifier: TypeQualifier,
        pos: Location,
        value: Variant,
    ) -> Result<()> {
        self.parent.set_l_value_q(bare_name, qualifier, pos, value)
    }
}

//
// SubContext
//

impl<T: TypeResolver> SubContext<T> {
    //
    // LValue (e.g. X = ?, FOR X = ?)
    //

    fn set_l_value_q_parent(
        &mut self,
        n: QualifiedName,
        pos: Location,
        value: Variant,
    ) -> Result<()> {
        self.parent
            .set_l_value_q(n.bare_name(), n.qualifier(), pos, value)
    }

    fn do_insert_variable(
        &mut self,
        bare_name: CaseInsensitiveString,
        qualifier: TypeQualifier,
        value: Variant,
    ) {
        match self.variables.get_mut(&bare_name) {
            Some(inner_map) => {
                inner_map.insert(qualifier, Argument::ByVal(value));
            }
            None => {
                let mut inner_map: HashMap<TypeQualifier, Argument> = HashMap::new();
                inner_map.insert(qualifier, Argument::ByVal(value));
                self.variables.insert(bare_name, inner_map);
            }
        }
    }

    fn constant_exists_no_recursion<U: NameTrait>(&self, name_node: &U) -> bool {
        self.constants.contains_key(name_node.bare_name())
    }

    fn get_argument_mut(
        &mut self,
        bare_name: &CaseInsensitiveString,
        qualifier: TypeQualifier,
    ) -> Option<&mut Argument> {
        match self.variables.get_mut(bare_name) {
            Some(inner_map) => inner_map.get_mut(&qualifier),
            None => None,
        }
    }

    //
    // RValue
    //

    pub fn get_r_value(&self, name_node: &NameNode) -> Result<Option<Variant>> {
        let bare_name = name_node.bare_name();
        let opt_qualifier = name_node.opt_qualifier();
        let pos = name_node.location();
        self.get_r_value_q(bare_name, opt_qualifier, pos)
    }

    fn evaluate_argument(&self, arg: &Argument, pos: Location) -> Result<Option<Variant>> {
        match arg {
            Argument::ByVal(v) => Ok(Some(v.clone())),
            Argument::ByRef(n) => {
                self.parent
                    .get_r_value_q(n.bare_name(), Some(n.qualifier()), pos)
            }
        }
    }

    fn get_variable(
        &self,
        bare_name: &CaseInsensitiveString,
        qualifier: TypeQualifier,
    ) -> Option<&Argument> {
        match self.variables.get(bare_name) {
            Some(inner_map) => inner_map.get(&qualifier),
            None => None,
        }
    }

    //
    // Const LValue
    //

    pub fn set_const_l_value(&mut self, name_node: &NameNode, value: Variant) -> Result<()> {
        let pos = name_node.location();
        // subtle difference, bare name constants get their type from the value
        let bare_name: &CaseInsensitiveString;
        let casted: Variant;
        match name_node.as_ref() {
            Name::Bare(b) => {
                bare_name = b;
                casted = value;
            }
            Name::Qualified(q) => {
                bare_name = q.bare_name();
                let qualifier = q.qualifier();
                casted = do_cast(value, qualifier, pos)?;
            }
        }
        // if a local constant or parameter or variable already exists throw an error
        if self.constant_exists_no_recursion(name_node) || self.variables.contains_key(bare_name) {
            return Err(InterpreterError::new_with_pos("Duplicate definition", pos));
        }
        // set it
        self.constants.insert(bare_name.clone(), casted);
        Ok(())
    }

    //
    // Const RValue
    //

    // TODO why is this not used?

    pub fn get_const_r_value(&self, name_node: &NameNode) -> Result<Variant> {
        let bare_name = name_node.bare_name();
        let opt_qualifier = name_node.opt_qualifier();
        let pos = name_node.location();
        if self.variables.contains_key(bare_name) {
            return Err(InterpreterError::new_with_pos("Invalid constant", pos));
        }

        match self.get_constant(bare_name, opt_qualifier, pos)? {
            Some(v) => Ok(v.clone()),
            None => match self.get_parent_constant(bare_name, opt_qualifier, pos)? {
                Some(v) => Ok(v),
                None => Err(InterpreterError::new_with_pos(
                    "Invalid constant (undef variable)",
                    pos,
                )),
            },
        }
    }

    //
    // Get/Set function result
    //

    pub fn set_function_result(&mut self, v: Variant) {
        self.parent.set_function_result(v);
    }

    //
    // For built-in subs/functions
    //

    pub fn pop_front_unnamed(&mut self, pos: Location) -> Result<Variant> {
        self.try_pop_front_unnamed(pos).map(|opt| opt.unwrap())
    }

    pub fn try_pop_front_unnamed(&mut self, pos: Location) -> Result<Option<Variant>> {
        match self.unnamed_args.pop_front() {
            Some(arg) => {
                let v = self.evaluate_argument(&arg, pos)?;
                Ok(Some(v.unwrap().clone()))
            }
            None => Ok(None),
        }
    }

    pub fn pop_front_unnamed_arg(&mut self) -> Option<Argument> {
        self.unnamed_args.pop_front()
    }

    pub fn set_value_to_popped_arg(
        &mut self,
        arg: &Argument,
        value: Variant,
        pos: Location,
    ) -> Result<()> {
        match arg {
            Argument::ByVal(_) => panic!("Expected variable"),
            Argument::ByRef(n) => {
                let q = n.clone(); // clone to break duplicate borrow
                self.set_l_value_q_parent(q, pos, value)
            }
        }
    }
}

//
// SubContext traits
//

impl<T: TypeResolver> TypeResolver for SubContext<T> {
    fn resolve<U: NameTrait>(&self, n: &U) -> TypeQualifier {
        self.parent.resolve(n)
    }
}

impl<T: TypeResolver> CreateParameter for SubContext<T> {
    fn create_parameter(&mut self, name_node: &NameNode) -> Result<Argument> {
        let bare_name = name_node.bare_name();
        let opt_qualifier = name_node.opt_qualifier();
        let pos = name_node.location();

        match self.get_constant(bare_name, opt_qualifier, pos)? {
            Some(v) => Ok(Argument::ByVal(v.clone())),
            None => {
                let qualifier = match opt_qualifier {
                    Some(q) => q,
                    None => self.resolve(bare_name),
                };

                // variable?
                match self.get_variable(bare_name, qualifier) {
                    // ref pointing to var
                    Some(_) => Ok(Argument::ByRef(QualifiedName::new(
                        bare_name.clone(),
                        qualifier,
                    ))),
                    None => {
                        // parent constant?
                        match self.get_parent_constant(bare_name, opt_qualifier, pos)? {
                            Some(v) => Ok(Argument::ByVal(v.clone())),
                            None => {
                                // create the variable in this scope
                                // e.g. INPUT N
                                self.do_insert_variable(
                                    bare_name.clone(),
                                    qualifier,
                                    Variant::default_variant(qualifier),
                                );
                                Ok(Argument::ByRef(QualifiedName::new(
                                    bare_name.clone(),
                                    qualifier,
                                )))
                            }
                        }
                    }
                }
            }
        }
    }
}

impl<T: TypeResolver> SetLValueQ for SubContext<T> {
    fn set_l_value_q(
        &mut self,
        bare_name: &CaseInsensitiveString,
        qualifier: TypeQualifier,
        pos: Location,
        value: Variant,
    ) -> Result<()> {
        if self.constant_exists_no_recursion(bare_name) {
            return Err(InterpreterError::new_with_pos("Duplicate definition", pos));
        }

        // if a parameter exists, set it (might be a ref)
        match self.get_argument_mut(bare_name, qualifier) {
            Some(a) => {
                match a {
                    Argument::ByVal(_) => {
                        let casted = do_cast(value, qualifier, pos)?;
                        *a = Argument::ByVal(casted);
                        Ok(())
                    }
                    Argument::ByRef(n) => {
                        let q = n.clone(); // clone needed to break duplicate borrow
                        self.set_l_value_q_parent(q, pos, value)
                    }
                }
            }
            None => {
                // A parameter does not exist. Create/Update a variable.
                let casted = do_cast(value, qualifier, pos)?;
                self.do_insert_variable(bare_name.clone(), qualifier, casted);
                Ok(())
            }
        }
    }
}

//
// Context
//

impl<T: TypeResolver> Context<T> {
    pub fn new(resolver: Rc<RefCell<T>>) -> Self {
        Self::Root(RootContext::new(resolver))
    }

    pub fn push_args_context(self) -> Self {
        Self::Args(ArgsContext {
            parent: Box::new(self),
            args: (HashMap::new(), VecDeque::new()),
        })
    }

    pub fn swap_args_with_sub_context(self) -> Self {
        match self {
            Self::Args(a) => Self::Sub(SubContext {
                parent: a.parent,
                variables: a.args.0,
                constants: HashMap::new(),
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

    // adapter methods

    pub fn get_r_value<U: NameTrait + HasLocation>(
        &self,
        name_node: &U,
    ) -> Result<Option<Variant>> {
        self.get_r_value_q(
            name_node.bare_name(),
            name_node.opt_qualifier(),
            name_node.location(),
        )
    }

    pub fn set_l_value<U: NameTrait>(
        &mut self,
        name: &U,
        pos: Location,
        value: Variant,
    ) -> Result<()> {
        self.set_l_value_q(name.bare_name(), self.resolve(name), pos, value)
    }

    pub fn set_const_l_value(&mut self, name_node: &NameNode, value: Variant) -> Result<()> {
        match self {
            Self::Root(r) => r.set_const_l_value(name_node, value),
            Self::Sub(s) => s.set_const_l_value(name_node, value),
            _ => panic!("Not allowed in an arg context"),
        }
    }

    pub fn demand_args(&mut self) -> &mut ArgsContext<T> {
        match self {
            Self::Args(a) => a,
            _ => panic!("Not in an args context"),
        }
    }

    pub fn demand_sub(&mut self) -> &mut SubContext<T> {
        match self {
            Self::Sub(s) => s,
            _ => panic!("Not in a subprogram context"),
        }
    }

    pub fn set_function_result(&mut self, v: Variant) {
        match self {
            Self::Root(r) => r.function_result = v,
            Self::Args(a) => a.parent.set_function_result(v),
            Self::Sub(s) => s.parent.set_function_result(v),
        }
    }

    pub fn get_function_result(&self) -> &Variant {
        match self {
            Self::Root(r) => &r.function_result,
            Self::Args(a) => a.parent.get_function_result(),
            Self::Sub(s) => s.parent.get_function_result(),
        }
    }
}

//
// Context traits
//

impl<T: TypeResolver> TypeResolver for Context<T> {
    fn resolve<U: NameTrait>(&self, n: &U) -> TypeQualifier {
        match self {
            Self::Root(r) => r.resolve(n),
            Self::Sub(s) => s.resolve(n),
            Self::Args(a) => a.resolve(n),
        }
    }
}

impl<T: TypeResolver> CreateParameter for Context<T> {
    fn create_parameter(&mut self, name_node: &NameNode) -> Result<Argument> {
        match self {
            Self::Root(r) => r.create_parameter(name_node),
            Self::Sub(s) => s.create_parameter(name_node),
            Self::Args(a) => a.create_parameter(name_node),
        }
    }
}

impl<T: TypeResolver> SetLValueQ for Context<T> {
    fn set_l_value_q(
        &mut self,
        bare_name: &CaseInsensitiveString,
        qualifier: TypeQualifier,
        pos: Location,
        value: Variant,
    ) -> Result<()> {
        match self {
            Self::Root(r) => r.set_l_value_q(bare_name, qualifier, pos, value),
            Self::Sub(s) => s.set_l_value_q(bare_name, qualifier, pos, value),
            Self::Args(a) => a.set_l_value_q(bare_name, qualifier, pos, value),
        }
    }
}
