use super::{InterpreterError, Result};
use crate::casting::cast;
use crate::common::{CaseInsensitiveString, HasLocation, Location};
use crate::instruction_generator::NamedRefParam;
use crate::linter::*;
use crate::variant::Variant;
use std::collections::{HashMap, VecDeque};

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

// TODO review how much is needed after linter, run code coverage

fn do_cast(value: Variant, qualifier: TypeQualifier, pos: Location) -> Result<Variant> {
    cast(value, qualifier).map_err(|e| InterpreterError::new_with_pos(e, pos))
}

pub type VariableMap = HashMap<CaseInsensitiveString, HashMap<TypeQualifier, Variant>>;
pub type ConstantMap = HashMap<CaseInsensitiveString, Variant>;
pub type ArgumentMap = HashMap<CaseInsensitiveString, HashMap<TypeQualifier, Argument>>;
pub type UnnamedArgs = VecDeque<Argument>;
pub type Args = (ArgumentMap, UnnamedArgs);

#[derive(Debug)]
pub struct RootContext {
    variables: VariableMap,
    constants: ConstantMap,
    function_result: Variant,
}

#[derive(Debug)]
pub struct ArgsContext {
    parent: Box<Context>,
    args: Args,
}

#[derive(Debug)]
pub struct SubContext {
    parent: Box<Context>,
    variables: ArgumentMap,
    constants: ConstantMap,
    unnamed_args: UnnamedArgs,
}

#[derive(Debug)]
pub enum Context {
    Root(RootContext),
    Sub(SubContext),
    Args(ArgsContext),
}

trait CreateParameter {
    fn create_parameter(&mut self, name: QualifiedName) -> Argument;
}

pub trait SetLValueQ {
    fn set_l_value_q(&mut self, name: QualifiedName, pos: Location, value: Variant) -> Result<()>;
}

trait GetConstant {
    fn get_constant(&self, name: &QualifiedName) -> Option<&Variant>;
}

impl GetConstant for ConstantMap {
    fn get_constant(&self, name: &QualifiedName) -> Option<&Variant> {
        match self.get(name.bare_name()) {
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
}

impl GetConstant for RootContext {
    fn get_constant(&self, name: &QualifiedName) -> Option<&Variant> {
        self.constants.get_constant(name)
    }
}

impl GetConstant for SubContext {
    fn get_constant(&self, name: &QualifiedName) -> Option<&Variant> {
        self.constants.get_constant(name)
    }
}

impl GetConstant for Context {
    fn get_constant(&self, name: &QualifiedName) -> Option<&Variant> {
        match self {
            Self::Root(r) => r.get_constant(name),
            Self::Args(a) => a.parent.get_constant(name),
            Self::Sub(s) => s.get_constant(name),
        }
    }
}

trait GetParentConstant {
    fn get_parent_constant(&self, name: &QualifiedName) -> Option<Variant>;
}

impl GetParentConstant for RootContext {
    fn get_parent_constant(&self, _name: &QualifiedName) -> Option<Variant> {
        None
    }
}

impl GetParentConstant for ArgsContext {
    fn get_parent_constant(&self, name: &QualifiedName) -> Option<Variant> {
        match self.parent.get_constant(name) {
            Some(v) => Some(v.clone()),
            None => self.parent.get_parent_constant(name),
        }
    }
}

impl GetParentConstant for SubContext {
    fn get_parent_constant(&self, name: &QualifiedName) -> Option<Variant> {
        match self.parent.get_constant(name) {
            Some(v) => Some(v.clone()),
            None => self.parent.get_parent_constant(name),
        }
    }
}

impl GetParentConstant for Context {
    fn get_parent_constant(&self, name: &QualifiedName) -> Option<Variant> {
        match self {
            Self::Root(r) => r.get_parent_constant(name),
            Self::Args(a) => a.get_parent_constant(name),
            Self::Sub(s) => s.get_parent_constant(name),
        }
    }
}

trait GetRValueQualified {
    fn get_r_value_q(&self, name: &QualifiedName) -> Option<Variant>;
}

impl GetRValueQualified for RootContext {
    fn get_r_value_q(&self, name: &QualifiedName) -> Option<Variant> {
        // local constant?
        match self.constants.get_constant(name) {
            Some(v) => Some(v.clone()),
            None => {
                // variable?
                match self.get_variable(name) {
                    Some(v) => Some(v.clone()),
                    None => None,
                }
            }
        }
    }
}

impl GetRValueQualified for ArgsContext {
    fn get_r_value_q(&self, name: &QualifiedName) -> Option<Variant> {
        self.parent.get_r_value_q(name)
    }
}

impl GetRValueQualified for SubContext {
    fn get_r_value_q(&self, name: &QualifiedName) -> Option<Variant> {
        // local constant?
        match self.get_constant(name) {
            Some(v) => Some(v.clone()),
            None => {
                // variable?
                match self.get_variable(name) {
                    Some(v) => self.evaluate_argument(v),
                    None => {
                        // parent constant?
                        self.get_parent_constant(name)
                    }
                }
            }
        }
    }
}

impl GetRValueQualified for Context {
    fn get_r_value_q(&self, name: &QualifiedName) -> Option<Variant> {
        match self {
            Self::Root(r) => r.get_r_value_q(name),
            Self::Args(a) => a.get_r_value_q(name),
            Self::Sub(s) => s.get_r_value_q(name),
        }
    }
}

//
// RootContext
//

impl RootContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            constants: HashMap::new(),
            function_result: Variant::VInteger(0),
        }
    }

    //
    // LValue (e.g. X = ?, FOR X = ?)
    //

    fn do_insert_variable(&mut self, name: QualifiedName, value: Variant) {
        match self.variables.get_mut(name.bare_name()) {
            Some(inner_map) => {
                inner_map.insert(name.qualifier(), value);
            }
            None => {
                let mut inner_map: HashMap<TypeQualifier, Variant> = HashMap::new();
                let (bare_name, qualifier) = name.consume();
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

    fn get_variable(&self, name: &QualifiedName) -> Option<&Variant> {
        match self.variables.get(name.bare_name()) {
            Some(inner_map) => inner_map.get(&name.qualifier()),
            None => None,
        }
    }

    //
    // Const LValue
    //

    pub fn set_const_l_value(&mut self, name_node: &QNameNode, value: Variant) -> Result<()> {
        let pos = name_node.location();
        // subtle difference, bare name constants get their type from the value
        let bare_name: &CaseInsensitiveString = name_node.bare_name();
        let qualifier = name_node.qualifier();
        let casted: Variant = do_cast(value, qualifier, pos)?;
        // if a local constant or parameter or variable already exists throw an error
        if self.constant_exists_no_recursion(name_node) || self.variables.contains_key(bare_name) {
            return Err(InterpreterError::new_with_pos("Duplicate definition", pos));
        }
        // set it
        self.constants.insert(bare_name.clone(), casted);
        Ok(())
    }
}

//
// RootContext traits
//

impl CreateParameter for RootContext {
    fn create_parameter(&mut self, name: QualifiedName) -> Argument {
        match self.get_constant(&name) {
            Some(v) => Argument::ByVal(v.clone()),
            None => {
                match self.get_variable(&name) {
                    // ref pointing to var
                    Some(_) => Argument::ByRef(name),
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

impl SetLValueQ for RootContext {
    fn set_l_value_q(&mut self, name: QualifiedName, pos: Location, value: Variant) -> Result<()> {
        // if a constant exists, throw error
        if self.constant_exists_no_recursion(&name) {
            return Err(InterpreterError::new_with_pos("Duplicate definition", pos));
        }
        // Arguments do not exist at root level. Create/Update a variable.
        let casted = do_cast(value, name.qualifier(), pos)?;
        self.do_insert_variable(name, casted);
        Ok(())
    }
}

//
// ArgsContext
//

impl ArgsContext {
    pub fn push_back_unnamed_ref_parameter(&mut self, name: QualifiedName) {
        let arg = self.create_parameter(name);
        self.args.1.push_back(arg);
    }

    pub fn push_back_unnamed_val_parameter(&mut self, value: Variant) {
        self.args.1.push_back(Argument::ByVal(value));
    }

    pub fn set_named_ref_parameter(&mut self, named_ref_param: &NamedRefParam) {
        let arg = self.create_parameter(named_ref_param.argument_name.clone());
        self.insert_next_argument(&named_ref_param.parameter_name, arg);
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

impl CreateParameter for ArgsContext {
    fn create_parameter(&mut self, name: QualifiedName) -> Argument {
        self.parent.create_parameter(name)
    }
}

impl SetLValueQ for ArgsContext {
    fn set_l_value_q(&mut self, name: QualifiedName, pos: Location, value: Variant) -> Result<()> {
        self.parent.set_l_value_q(name, pos, value)
    }
}

//
// SubContext
//

impl SubContext {
    //
    // LValue (e.g. X = ?, FOR X = ?)
    //

    fn set_l_value_q_parent(
        &mut self,
        n: QualifiedName,
        pos: Location,
        value: Variant,
    ) -> Result<()> {
        self.parent.set_l_value_q(n, pos, value)
    }

    fn do_insert_variable(&mut self, name: QualifiedName, value: Variant) {
        match self.variables.get_mut(name.bare_name()) {
            Some(inner_map) => {
                inner_map.insert(name.qualifier(), Argument::ByVal(value));
            }
            None => {
                let mut inner_map: HashMap<TypeQualifier, Argument> = HashMap::new();
                let (bare_name, qualifier) = name.consume();
                inner_map.insert(qualifier, Argument::ByVal(value));
                self.variables.insert(bare_name, inner_map);
            }
        }
    }

    fn constant_exists_no_recursion<U: NameTrait>(&self, name_node: &U) -> bool {
        self.constants.contains_key(name_node.bare_name())
    }

    fn get_argument_mut(&mut self, name: &QualifiedName) -> Option<&mut Argument> {
        match self.variables.get_mut(name.bare_name()) {
            Some(inner_map) => inner_map.get_mut(&name.qualifier()),
            None => None,
        }
    }

    //
    // RValue
    //

    fn evaluate_argument(&self, arg: &Argument) -> Option<Variant> {
        match arg {
            Argument::ByVal(v) => Some(v.clone()),
            Argument::ByRef(n) => self.parent.get_r_value_q(n),
        }
    }

    fn get_variable(&self, name: &QualifiedName) -> Option<&Argument> {
        match self.variables.get(name.bare_name()) {
            Some(inner_map) => inner_map.get(&name.qualifier()),
            None => None,
        }
    }

    //
    // Const LValue
    //

    pub fn set_const_l_value(&mut self, name_node: &QNameNode, value: Variant) -> Result<()> {
        let pos = name_node.location();
        // subtle difference, bare name constants get their type from the value
        let bare_name: &CaseInsensitiveString = name_node.bare_name();
        let qualifier = name_node.qualifier();
        let casted: Variant = do_cast(value, qualifier, pos)?;
        // if a local constant or parameter or variable already exists throw an error
        if self.constant_exists_no_recursion(name_node) || self.variables.contains_key(bare_name) {
            return Err(InterpreterError::new_with_pos("Duplicate definition", pos));
        }
        // set it
        self.constants.insert(bare_name.clone(), casted);
        Ok(())
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

impl CreateParameter for SubContext {
    fn create_parameter(&mut self, name: QualifiedName) -> Argument {
        match self.get_constant(&name) {
            Some(v) => Argument::ByVal(v.clone()),
            None => {
                // variable?
                match self.get_variable(&name) {
                    // ref pointing to var
                    Some(_) => Argument::ByRef(name),
                    None => {
                        // parent constant?
                        match self.get_parent_constant(&name) {
                            Some(v) => Argument::ByVal(v.clone()),
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
}

impl SetLValueQ for SubContext {
    fn set_l_value_q(&mut self, name: QualifiedName, pos: Location, value: Variant) -> Result<()> {
        if self.constant_exists_no_recursion(&name) {
            return Err(InterpreterError::new_with_pos("Duplicate definition", pos));
        }

        // if a parameter exists, set it (might be a ref)
        match self.get_argument_mut(&name) {
            Some(a) => {
                match a {
                    Argument::ByVal(_) => {
                        let casted = do_cast(value, name.qualifier(), pos)?;
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
                let casted = do_cast(value, name.qualifier(), pos)?;
                self.do_insert_variable(name, casted);
                Ok(())
            }
        }
    }
}

//
// Context
//

impl Context {
    pub fn new() -> Self {
        Self::Root(RootContext::new())
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

    pub fn get_r_value(&self, name_node: &QNameNode) -> Option<Variant> {
        self.get_r_value_q(name_node.as_ref())
    }

    pub fn set_const_l_value(&mut self, name_node: &QNameNode, value: Variant) -> Result<()> {
        match self {
            Self::Root(r) => r.set_const_l_value(name_node, value),
            Self::Sub(s) => s.set_const_l_value(name_node, value),
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

impl CreateParameter for Context {
    fn create_parameter(&mut self, name: QualifiedName) -> Argument {
        match self {
            Self::Root(r) => r.create_parameter(name),
            Self::Sub(s) => s.create_parameter(name),
            Self::Args(a) => a.create_parameter(name),
        }
    }
}

impl SetLValueQ for Context {
    fn set_l_value_q(&mut self, name: QualifiedName, pos: Location, value: Variant) -> Result<()> {
        match self {
            Self::Root(r) => r.set_l_value_q(name, pos, value),
            Self::Sub(s) => s.set_l_value_q(name, pos, value),
            Self::Args(a) => a.set_l_value_q(name, pos, value),
        }
    }
}
