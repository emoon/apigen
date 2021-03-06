//use heck::ToSnakeCase;
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fs::File,
    io::{Read, Write},
    path::Path,
};
use thiserror::Error;

//#[cfg(debug_assertions)]
const _GRAMMAR: &str = include_str!("api.pest");

///
/// Current primitive types
///
const PRMITIVE_TYPES: &[&str] = &[
    "void", "i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64", "bool", "f32", "f64",
];

///
/// Variable type
///
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum VariableType {
    None,
    /// Self (aka this pointer in C++ and self in Rust)
    SelfType,
    /// Enum type
    Enum,
    /// Struct/other type
    Regular,
    /// String type
    Str,
    /// Prmitive type (such as i32,u64,etc)
    Primitive,
}

///
/// Array Type
///
#[derive(PartialEq, Debug, Clone)]
pub enum ArrayType {
    /// Array is unsized
    Unsized,
    /// Array with fixed size
    SizedArray(String),
}

impl Default for ArrayType {
    fn default() -> ArrayType {
        ArrayType::Unsized
    }
}

/// Set if the type has a modifier on it (mutable pointer, const pointer or reference)
#[derive(PartialEq, Debug, Clone)]
pub enum TypeModifier {
    // No modifier on the type
    None,
    // const pointer (i.e *const <type>)
    ConstPointer,
    // const pointer (i.e *<type>)
    MutPointer,
    // Refernce (i.e &type)
    Reference,
}

/// Holds the data for a variable. It's name and it's type and additional flags
#[derive(Debug, Clone)]
pub struct Variable {
    /// Documentation
    pub doc_comments: Vec<String>,
    /// Which def file this variable comes from
    pub def_file: String,
    /// Name of the variable
    pub name: String,
    /// Type of the variable
    pub vtype: VariableType,
    /// Name of the variable type
    pub type_name: String,
    /// Name of the variable type
    pub default_value: String,
    /// Type of enum
    pub enum_type: EnumType,
    /// If variable is an array
    pub array: Option<ArrayType>,
    /// If the variable has a type modifier (such as pointer, ref, etc)
    pub type_modifier: TypeModifier,
    /// If variable is optional (nullable)
    pub optional: bool,
}

/// Default implementation for Variable
impl Default for Variable {
    fn default() -> Self {
        Variable {
            name: String::new(),
            doc_comments: Vec::new(),
            def_file: String::new(),
            vtype: VariableType::None,
            type_name: String::new(),
            enum_type: EnumType::Regular,
            default_value: String::new(),
            array: None,
            optional: false,
            type_modifier: TypeModifier::None,
        }
    }
}

///
/// Function type
///
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FunctionType {
    /// This is a regular function
    Regular,
    /// Static function
    Static,
    /// Function that is manually implemented in some cases
    Manual,
}

///
/// Holds the data for a function. Name, function_args, return_type, etc
///
#[derive(Debug, Clone)]
pub struct Function {
    /// Documentation
    pub doc_comments: Vec<String>,
    /// Which def file this function comes from
    pub def_file: String,
    /// Name of the function
    pub name: String,
    /// Function argumnts
    pub function_args: Vec<Variable>,
    /// Return value
    pub return_val: Option<Variable>,
    /// Type of function. See FunctionType descrition for more info
    pub func_type: FunctionType,
}

/// Default implementation for Function
impl Default for Function {
    fn default() -> Self {
        Function {
            doc_comments: Vec::new(),
            name: String::new(),
            def_file: String::new(),
            function_args: Vec::new(),
            return_val: None,
            func_type: FunctionType::Regular,
        }
    }
}

/// Holds the data for a struct
#[derive(Debug, Default)]
pub struct Struct {
    /// Docummentanion
    pub doc_comments: Vec<String>,
    /// Name
    pub name: String,
    /// Which def file this struct comes from
    pub def_file: String,
    /// Variables in the struct
    pub variables: Vec<Variable>,
    /// Functions for the struct
    pub functions: Vec<Function>,
    /// Attributes of thu struct
    pub attributes: Vec<String>,
    /// Traits
    pub traits: Vec<String>,
    /// List of derives
    pub derives: Vec<String>,
}

/// C/C++ style enum
#[derive(Debug)]
pub struct EnumEntry {
    /// Documentation
    pub doc_comments: Vec<String>,
    /// Name of the enum entry
    pub name: String,
    /// Value of the enum entry
    pub value: u64,
}

/// Enums in C++ can have same value for different enum ids. This isn't supported in Rust.
/// Also Rust doesn't support that your "or" enums flags so we need to handle that.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EnumType {
    /// All values are in sequantial order and no overlap
    Regular,
    /// This enum is constructed with bitflags due to being power of two or overlapping values
    Bitflags,
}

impl Default for EnumType {
    fn default() -> EnumType {
        EnumType::Regular
    }
}

/// Enum type
#[derive(Debug, Default)]
pub struct Enum {
    /// Documentation
    pub doc_comments: Vec<String>,
    /// Name of the enum
    pub name: String,
    /// The file this enum is present in
    pub def_file: String,
    /// Type of enum
    pub enum_type: EnumType,
    /// Qt supports having a flags macro on enums being type checked with an extra name
    pub flags_name: String,
    /// All the enem entries
    pub entries: Vec<EnumEntry>,
}

// Type type
#[derive(Debug, Default)]
pub struct Type {
    /// Documentation
    pub doc_comments: Vec<String>,
    /// Variable that includes type and name
    pub var: Variable,
}

// Union type
#[derive(Debug, Default)]
pub struct Const {
    /// Documentation
    pub doc_comments: Vec<String>,
    /// Name of the type
    pub name: String,
    /// Data
    pub value: String,
}

/// Api definition for a file
#[derive(Debug, Default)]
pub struct ApiDef {
    /// full filename path
    pub filename: String,
    /// Base filename (such as foo/file/some_name.def) is some_name
    pub base_filename: String,
    /// Mods to to be included in the file
    pub mods: Vec<String>,
    /// Callbacks types
    pub callbacks: Vec<Function>,
    /// Structs that only holds data
    pub structs: Vec<Struct>,
    /// Enums
    pub enums: Vec<Enum>,
    /// Types
    pub types: Vec<Type>,
    /// Unions
    pub unions: Vec<Struct>,
    /// Consts
    pub consts: Vec<Const>,
}

#[derive(Error, Debug)]
pub enum ApigenError {
    #[error("data store disconnected")]
    Disconnect(#[from] std::io::Error),
    #[error("the data for key `{0}` is not available")]
    Redaction(String),
    #[error("invalid header (expected {expected:?}, found {found:?})")]
    InvalidHeader { expected: String, found: String },
    #[error("unknown data store error")]
    Unknown,
}

pub type Result<T> = std::result::Result<T, ApigenError>;

/// Checks if name is a primitive
fn is_primitve(name: &str) -> bool {
    PRMITIVE_TYPES.iter().any(|&type_name| type_name == name)
}

#[derive(Parser)]
#[grammar = "api.pest"]
pub struct ApiParser;

/// Build struct info for a parsed API def file
impl ApiParser {
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<ApiDef> {
        let mut buffer = String::new();
        let mut f = File::open(&path)?;
        f.read_to_string(&mut buffer)?;
        Self::parse_string(&buffer, path.as_ref().to_str().unwrap())
    }

    pub fn parse_string(buffer: &str, filename: &str) -> Result<ApiDef> {
        let mut api_def = ApiDef::default();

        let chunks = ApiParser::parse(Rule::chunk, buffer)
            .unwrap_or_else(|e| panic!("APiParser: {} {}", filename, e));

        if let Some(base_name) = Path::new(filename).file_stem() {
            let base_filename = base_name.to_str().unwrap();
            api_def.filename = filename.to_owned();
            api_def.base_filename = base_filename.to_owned();
        }

        let mut current_comments = Vec::new();

        for chunk in chunks {
            match chunk.as_rule() {
                Rule::structdef => {
                    let sdef = Self::fill_struct(chunk, &current_comments, &api_def.base_filename);
                    current_comments.clear();

                    // If we have some variables in the struct we push it to pod_struct
                    api_def.structs.push(sdef);
                }

                Rule::callbackdef => {
                    let mut func = Self::fill_callback(chunk, &current_comments);
                    func.func_type = FunctionType::Static;
                    api_def.callbacks.push(func);
                    current_comments.clear();
                }

                Rule::moddef => {
                    for entry in chunk.into_inner() {
                        if entry.as_rule() == Rule::name {
                            api_def.mods.push(entry.as_str().to_owned())
                        }
                    }
                }

                Rule::type_value => {
                    let mut type_value = Type::default();

                    for entry in chunk.into_inner() {
                        if entry.as_rule() == Rule::var {
                            type_value.var = Self::get_variable(entry, &current_comments);
                        }
                    }

                    api_def.types.push(type_value);

                    current_comments.clear();
                }

                Rule::const_value => {
                    let mut const_value = Const::default();

                    for entry in chunk.into_inner() {
                        match entry.as_rule() {
                            Rule::name => const_value.name = entry.as_str().to_owned(),
                            Rule::name_or_num => const_value.value = entry.as_str().to_owned(),
                            Rule::raw_string => const_value.value = entry.as_str().to_owned(),
                            _ => (),
                        }
                    }

                    api_def.consts.push(const_value);
                }

                Rule::doc_comment => {
                    current_comments.push(chunk.as_str()[4..].to_owned());
                }

                Rule::enumdef => {
                    let mut enum_def = Enum {
                        def_file: "".to_owned(), // TODO: fixme
                        doc_comments: current_comments.to_owned(),
                        ..Default::default()
                    };
                    current_comments.clear();

                    for entry in chunk.into_inner() {
                        match entry.as_rule() {
                            Rule::name => enum_def.name = entry.as_str().to_owned(),
                            Rule::fieldlist => enum_def.entries = Self::fill_field_list_enum(entry),
                            Rule::enum_flags => {
                                enum_def.flags_name = entry
                                    .into_inner()
                                    .next()
                                    .map(|e| e.as_str())
                                    .unwrap()
                                    .to_owned();
                            }
                            _ => (),
                        }
                    }

                    // Figure out enum type
                    enum_def.enum_type = Self::determine_enum_type(&enum_def);
                    api_def.enums.push(enum_def);
                }

                Rule::uniondef => {
                    let union_def =
                        Self::fill_struct(chunk, &current_comments, &api_def.base_filename);
                    current_comments.clear();
                    api_def.unions.push(union_def);
                }

                _ => (),
            }
        }

        Ok(api_def)
    }

    /// Check if the enum values are in a single sequnce
    fn check_sequential(enum_def: &Enum) -> bool {
        if enum_def.entries.is_empty() {
            return false;
        }

        let mut current = enum_def.entries[0].value;

        for e in &enum_def.entries {
            if current != e.value {
                return false;
            }

            current += 1;
        }

        true
    }

    /// Check if the enum values overlaps
    fn check_overlapping(enum_def: &Enum) -> bool {
        let mut values = HashSet::<u64>::new();

        for v in &enum_def.entries {
            if values.contains(&v.value) {
                return true;
            } else {
                values.insert(v.value);
            }
        }

        false
    }

    /// check if an enum only has power of two values in it. This function calculate in percent how
    /// many values that happens to be power of two and returns true if it's a above a certain
    /// threshold. The reason for this is that some enums also combinations of other values
    /// so it's not possible to *only* check for single power of two values.
    fn check_power_of_two(enum_def: &Enum) -> bool {
        if enum_def.entries.is_empty() {
            return false;
        }

        let power_of_two_count: u32 = enum_def
            .entries
            .iter()
            .filter(|e| e.value.is_power_of_two())
            .map(|_v| 1)
            .sum();

        // if we have >= 50% of power of two values assume this enum is being used as bitflags
        let percent = power_of_two_count as f32 / enum_def.entries.len() as f32;
        percent > 0.5
    }

    /// Figures out the type of enum
    fn determine_enum_type(enum_def: &Enum) -> EnumType {
        // if all number is in a single linear sequence. This currently misses if
        // valid "breaks" in sequences
        let sequential = Self::check_sequential(enum_def);
        // if all numbers aren't overlapping
        let overlapping = Self::check_overlapping(enum_def);
        // check if all values are power of two
        let power_of_two = Self::check_power_of_two(enum_def);

        // If enum is sequential and has no overlapping we can use it as a regular enum
        if sequential && !overlapping {
            return EnumType::Regular;
        }

        // if all values are power of two we assume this should be used as bitfield
        // or has overlapping values we
        if power_of_two || overlapping {
            EnumType::Bitflags
        } else {
            EnumType::Regular
        }
    }

    fn fill_callback(chunk: Pair<Rule>, doc_comments: &[String]) -> Function {
        let mut func = Function::default();

        for entry in chunk.into_inner() {
            if entry.as_rule() == Rule::function {
                func = Self::get_function(entry, doc_comments);
            }
        }

        func
    }

    /// Fill struct def
    fn fill_struct(chunk: Pair<Rule>, doc_comments: &[String], def_file: &str) -> Struct {
        let mut sdef = Struct {
            doc_comments: doc_comments.to_owned(),
            def_file: def_file.to_owned(),
            ..Default::default()
        };

        for entry in chunk.into_inner() {
            match entry.as_rule() {
                Rule::name => sdef.name = entry.as_str().to_owned(),
                Rule::attributes => sdef.attributes = Self::get_attrbutes(entry),
                Rule::derive => sdef.derives = Self::get_attrbutes(entry),
                Rule::traits => sdef.traits = Self::get_attrbutes(entry),
                Rule::fieldlist => {
                    let (var_entries, func_entries) = Self::fill_field_list(entry);
                    sdef.variables = var_entries;
                    sdef.functions = func_entries;
                }

                _ => (),
            }
        }

        sdef
    }

    /// Get attributes for a struct
    fn get_attrbutes(rule: Pair<Rule>) -> Vec<String> {
        let mut attribs = Vec::new();
        for entry in rule.into_inner() {
            if entry.as_rule() == Rule::namelist {
                attribs = Self::get_namelist_list(entry);
            }
        }

        attribs
    }

    /// collect namelist (array) of strings
    fn get_namelist_list(rule: Pair<Rule>) -> Vec<String> {
        rule.into_inner().map(|e| e.as_str().to_owned()).collect()
    }

    /// Fill the entries in a struct
    /// Returns tuple with two ararys for variables and functions
    fn fill_field_list(rule: Pair<Rule>) -> (Vec<Variable>, Vec<Function>) {
        let mut var_entries = Vec::new();
        let mut func_entries = Vec::new();
        let mut doc_comments = Vec::new();

        for entry in rule.into_inner() {
            match entry.as_rule() {
                Rule::field => {
                    let field = entry.clone().into_inner().next().unwrap();

                    match field.as_rule() {
                        Rule::var => {
                            var_entries.push(Self::get_variable(field, &doc_comments));
                            doc_comments.clear();
                        }
                        Rule::function => {
                            func_entries.push(Self::get_function(field, &doc_comments));
                            doc_comments.clear();
                        }
                        _ => (),
                    }
                }

                Rule::doc_comment => {
                    if entry.as_str().len() >= 4 {
                        doc_comments.push(entry.as_str()[4..].to_owned());
                    }
                }

                _ => (),
            }
        }

        (var_entries, func_entries)
    }

    ///
    /// Get data for function declaration
    ///
    fn get_function(rule: Pair<Rule>, doc_comments: &[String]) -> Function {
        let mut is_static_func = false;
        let mut function = Function {
            doc_comments: doc_comments.to_owned(),
            ..Function::default()
        };

        for entry in rule.into_inner() {
            match entry.as_rule() {
                Rule::name => function.name = entry.as_str().to_owned(),
                Rule::manual_typ => function.func_type = FunctionType::Manual,
                Rule::varlist => {
                    function.function_args = Self::get_variable_list(entry, is_static_func)
                }
                Rule::retexp => function.return_val = Some(Self::get_variable(entry, &Vec::new())),
                Rule::static_typ => {
                    function.func_type = FunctionType::Static;
                    is_static_func = true;
                }
                _ => (),
            }
        }

        function
    }

    ///
    /// Gather variable list
    ///
    fn get_variable_list(rule: Pair<Rule>, is_static_func: bool) -> Vec<Variable> {
        let mut variables = if !is_static_func {
            vec![Variable {
                name: "self".to_owned(),
                vtype: VariableType::SelfType,
                ..Variable::default()
            }]
        } else {
            Vec::new()
        };

        let t = Vec::new();

        for entry in rule.into_inner() {
            variables.push(Self::get_variable(entry, &t));
        }

        variables
    }

    fn get_default_value(var: &mut Variable, rule: Pair<Rule>) {
        let mut default_value = String::new();
        for entry in rule.into_inner() {
            match entry.as_rule() {
                Rule::name_or_num => {
                    default_value = entry.as_str().to_owned();
                    break;
                }

                Rule::string => {
                    default_value = entry.as_str().to_owned();
                    break;
                }
                _ => (),
            }
        }

        var.default_value = default_value;
    }

    ///
    /// Get variable
    ///
    fn get_variable(rule: Pair<Rule>, doc_comments: &[String]) -> Variable {
        let mut vtype = Rule::var;
        let mut var = Variable::default();
        let mut type_name = String::new();

        var.doc_comments = doc_comments.to_owned();

        for entry in rule.into_inner() {
            match entry.as_rule() {
                Rule::name => var.name = entry.as_str().to_owned(),
                Rule::refexp => vtype = Rule::refexp,
                Rule::pointer_exp => vtype = Rule::pointer_exp,
                Rule::const_ptr_exp => vtype = Rule::const_ptr_exp,
                Rule::optional => var.optional = true,
                Rule::vtype => type_name = entry.as_str().to_owned(),
                Rule::default_val => Self::get_default_value(&mut var, entry),

                Rule::array => {
                    var.array = Some(ArrayType::Unsized);
                    // Get the type if we have an array
                    for entry in entry.into_inner() {
                        match entry.as_rule() {
                            Rule::vtype => type_name = entry.as_str().to_owned(),
                            Rule::refexp => vtype = Rule::refexp,
                            Rule::pointer_exp => vtype = Rule::pointer_exp,
                            Rule::const_ptr_exp => vtype = Rule::const_ptr_exp,
                            Rule::array_size => {
                                var.array = Some(ArrayType::SizedArray(
                                    entry.into_inner().as_str().to_owned(),
                                ));
                            }
                            _ => (),
                        }
                    }
                }

                _ => (),
            }
        }

        if !var.default_value.is_empty() {
            dbg!(&var.default_value);
        }

        // match up with the correct type
        let var_type = if type_name == "String" {
            VariableType::Str
        } else if is_primitve(&type_name) {
            VariableType::Primitive
        } else {
            VariableType::Regular
        };

        match vtype {
            Rule::pointer_exp => var.type_modifier = TypeModifier::MutPointer,
            Rule::const_ptr_exp => var.type_modifier = TypeModifier::MutPointer,
            Rule::refexp => var.type_modifier = TypeModifier::Reference,
            _ => (),
        }

        var.type_name = type_name;
        var.vtype = var_type;
        var
    }

    /// Get array of enums
    fn fill_field_list_enum(rule: Pair<Rule>) -> Vec<EnumEntry> {
        let mut entries = Vec::new();
        let mut doc_comments = Vec::new();

        for entry in rule.into_inner() {
            match entry.as_rule() {
                Rule::field => {
                    let field = entry.clone().into_inner().next().unwrap();

                    if field.as_rule() == Rule::enum_type {
                        entries.push(Self::get_enum(&doc_comments, field));
                        doc_comments.clear();
                    }
                }

                Rule::doc_comment => {
                    doc_comments.push(entry.as_str()[4..].to_owned());
                }

                _ => (),
            }
        }

        let mut counter = 0;

        for e in &mut entries {
            if e.value == u64::MAX {
                e.value = counter;
                counter += 1;
            } else {
                counter = e.value + 1;
            }
        }

        entries
    }

    /// Get enum
    fn get_enum(doc_comments: &[String], rule: Pair<Rule>) -> EnumEntry {
        let mut name = String::new();
        let mut assign = None;

        for entry in rule.into_inner() {
            match entry.as_rule() {
                Rule::name => name = entry.as_str().to_owned(),
                Rule::enum_assign => {
                    assign = Some(Self::get_enum_assign(entry).parse::<u64>().unwrap())
                }
                _ => (),
            }
        }

        if let Some(value) = assign {
            EnumEntry {
                doc_comments: doc_comments.to_owned(),
                name,
                value,
            }
        } else {
            EnumEntry {
                doc_comments: doc_comments.to_owned(),
                name,
                value: u64::MAX, // TODO: Reassigned at patchup
            }
        }
    }

    ///
    /// Get enum asign
    ///
    fn get_enum_assign(rule: Pair<Rule>) -> String {
        let mut name_or_num = String::new();

        for entry in rule.into_inner() {
            if entry.as_rule() == Rule::name_or_num {
                name_or_num = entry.as_str().to_owned();
                break;
            }
        }

        name_or_num
    }

    pub fn second_pass(api_defs: &mut [ApiDef]) {
        // TODO: Investigate if we actually need this pass
        // Build a hash map of all type and their types
        // and we also build two hashmaps for all types and which modules they belong into
        // and they are separate for structs and enums
        let mut type_def_file = HashMap::new();
        let mut enum_def_file_type = HashMap::new();
        let mut empty_structs = HashSet::new();

        for api_def in api_defs.iter() {
            api_def.structs.iter().for_each(|s| {
                if s.variables.is_empty() && !s.has_attribute("Handle") {
                    empty_structs.insert(s.name.to_owned());
                }
                type_def_file.insert(s.name.to_owned(), s.def_file.to_owned());
                type_def_file.insert(format!("{}Trait", s.name), s.def_file.to_owned());
            });

            api_def.enums.iter().for_each(|e| {
                enum_def_file_type.insert(e.name.to_owned(), (e.def_file.to_owned(), e.enum_type));

                if !e.flags_name.is_empty() {
                    enum_def_file_type.insert(
                        e.flags_name.to_owned(),
                        (e.def_file.to_owned(), EnumType::Bitflags),
                    );
                }
            });
        }

        for api_def in api_defs.iter_mut() {
            for s in &mut api_def.structs {
                for func in &mut s.functions {
                    for arg in &mut func.function_args {
                        if enum_def_file_type.contains_key(&arg.type_name) {
                            arg.vtype = VariableType::Enum;
                        }
                    }
                }
            }
        }
    }
}

impl ApiDef {
    // Generates the constast _C_MANUAL data to output and patches {CPrefix} with c_prefix input
    pub fn write_c_manual<W: Write>(&self, out: &mut W, c_prefix: &str) -> Result<()> {
        for c in &self.consts {
            if c.name != "_MANUAL_C" {
                continue;
            }

            let t = c.value.replace("{CPrefix}", c_prefix);
            write!(out, "\n{}\n", &t[1..t.len() - 1])?
        }

        Ok(())
    }
}

/// Impl for struct. Mostly helper functions to make it easier to extract info
impl Struct {
    /// Check if no wrapping class should be generated
    pub fn has_attribute(&self, attrib: &str) -> bool {
        self.attributes.iter().any(|s| s == attrib)
    }
}

/// Helper functions for function
impl Function {
    pub fn get_default_args(&self) -> Vec<&Variable> {
        self.function_args
            .iter()
            .filter(|arg| !arg.default_value.is_empty())
            .collect()
    }

    pub fn is_type_manual_static(&self) -> bool {
        self.func_type == FunctionType::Static || self.func_type == FunctionType::Manual
    }

    pub fn is_type_manual(&self) -> bool {
        self.func_type == FunctionType::Manual
    }

    pub fn is_type_static(&self) -> bool {
        self.func_type == FunctionType::Static
    }

    // Returns a list of funuction arguments for C function
    pub fn get_c_separated_arguments(&self, self_name: &str, c_prefix: &str) -> Vec<String> {
        let mut args = Vec::with_capacity(self.function_args.len());

        for arg in &self.function_args {
            match arg.vtype {
                VariableType::Str => args.push(format!("const char* {}", arg.name)),

                _ => match arg.array {
                    None => {
                        if arg.name != "va_args" && arg.type_name != "VA_ARGS" {
                            args.push(format!(
                                "{} {}",
                                arg.get_c_variable(self_name, c_prefix),
                                arg.name
                            ));
                        } else {
                            args.push("...".to_owned());
                        }
                    }

                    Some(ArrayType::Unsized) => {
                        args.push(format!(
                            "{}* {}",
                            arg.get_c_variable(self_name, c_prefix),
                            arg.name
                        ));
                        args.push(format!("uint64_t {}_size", arg.name));
                    }

                    Some(ArrayType::SizedArray(ref size)) => {
                        args.push(format!(
                            "{} {}[{}]",
                            arg.get_c_variable(self_name, c_prefix),
                            arg.name,
                            size
                        ));
                    }
                },
            }
        }

        args
    }

    pub fn get_c_arg_names(&self, self_name: &str) -> String {
        let mut output = String::with_capacity(256);

        for (i, arg) in self.function_args.iter().enumerate() {
            if i > 0 {
                output.push_str(", ")
            }

            if let Some(array_type) = arg.array.as_ref() {
                match array_type {
                    ArrayType::Unsized => {
                        output.push_str(&format!("{}, {}_size", arg.name, arg.name))
                    }
                    ArrayType::SizedArray(_size) => output.push_str(&arg.name),
                }
            } else if arg.vtype == VariableType::SelfType {
                output.push_str(self_name)
            } else {
                output.push_str(&arg.name);
            }
        }

        output
    }

    pub fn get_c_arguments(&self, self_name: &str, c_prefix: &str) -> String {
        let args = self.get_c_separated_arguments(self_name, c_prefix);

        let mut output = String::with_capacity(256);

        for (i, a) in args.iter().enumerate() {
            if i > 0 {
                output.push_str(", ")
            }

            output.push_str(a);
        }

        output
    }

    pub fn get_c_return_value(&self, c_prefix: &str) -> Cow<str> {
        if let Some(ret) = self.return_val.as_ref() {
            ret.get_c_variable("", c_prefix).into()
        } else {
            "void".into()
        }
    }
}

///
/// Impl for Variable. Helper functions to make C and Rust generation easier
///
impl Variable {
    pub fn get_c_primitive_type(&self) -> Cow<str> {
        let tname = self.type_name.as_str();

        match tname {
            "f32" => "float".into(),
            "bool" => "bool".into(),
            "f64" => "double".into(),
            "i32" => "int".into(),
            "void" => "void".into(),
            _ => {
                if self.type_name.starts_with('u') {
                    format!("uint{}_t", &tname[1..]).into()
                } else {
                    format!("int{}_t", &tname[1..]).into()
                }
            }
        }
    }

    pub fn get_c_variable(&self, self_type: &str, c_prefix: &str) -> String {
        let mut output = String::with_capacity(256);

        // TODO: If self type is a struct we should add struct at the front

        match self.type_modifier {
            TypeModifier::ConstPointer => output.push_str("const "),
            TypeModifier::Reference => output.push_str("const "),
            _ => (),
        }

        match self.vtype {
            VariableType::None => output.push_str("void"),
            VariableType::SelfType => output.push_str(&format!("struct {}{}", c_prefix, self_type)),
            VariableType::Regular => output.push_str(&format!("{}{}", c_prefix, self.type_name)),
            VariableType::Enum => output.push_str(&format!("{}{}", c_prefix, self.type_name)),
            VariableType::Str => output.push_str("const char*"),
            VariableType::Primitive => output.push_str(&self.get_c_primitive_type()),
        }

        match self.type_modifier {
            TypeModifier::ConstPointer => output.push('*'),
            TypeModifier::MutPointer => output.push('*'),
            TypeModifier::Reference => output.push('*'),
            _ => (),
        }

        output
    }

    pub fn get_primitive_type(&self) -> Cow<str> {
        let tname = self.type_name.as_str();

        match tname {
            "void" => "c_void".into(),
            _ => tname.into(),
        }
    }

    pub fn get_ffi_type(&self, self_type: &str) -> String {
        let mut output = String::with_capacity(256);

        match self.vtype {
            VariableType::None => output.push_str("c_void"),
            VariableType::SelfType => output.push_str(&format!("*mut {}", self_type)),
            VariableType::Regular => output.push_str(&self.type_name),
            VariableType::Enum => output.push_str(&self.type_name),
            VariableType::Str => output.push_str("*const c_char"),
            VariableType::Primitive => output.push_str(&self.get_primitive_type()),
        }

        match self.array.as_ref() {
            None => match self.type_modifier {
                TypeModifier::ConstPointer => format!("*const {}", output),
                TypeModifier::MutPointer => format!("*mut {}", output),
                TypeModifier::Reference => format!("*const {}", output),
                _ => output,
            },

            Some(ArrayType::Unsized) => {
                format!("*const {}, {}_size: u64", output, self.name)
            }

            Some(ArrayType::SizedArray(size)) => {
                format!("[{}; {}]", output, size)
            }
        }
    }

    pub fn get_c_struct_variable(&self, c_prefix: &str) -> String {
        let mut output = String::with_capacity(256);

        output.push_str(&format!("    {}", self.get_c_variable("", c_prefix)));

        // for arrays we generate a pointer and a size
        match self.array {
            None => output.push_str(&format!(" {};", self.name)),
            Some(ArrayType::Unsized) => {
                output.push_str(&format!("* {};\n", self.name));
                output.push_str(&format!("    uint64_t {}_size;", self.name));
            }

            Some(ArrayType::SizedArray(ref size)) => {
                output.push_str(&format!(" {}[{}];", self.name, size));
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_primitve_ok() {
        assert!(is_primitve("i32"));
    }

    #[test]
    fn test_primitve_false() {
        assert!(is_primitve("dummy"));
    }

    #[test]
    fn test_type() {
        let def = ApiParser::parse_string("type MetadataId: u64", "metadata.def").unwrap();
        assert_eq!(def.types.len(), 1);
        assert_eq!(def.types[0].var.name, "MetadataId");
        assert_eq!(def.types[0].var.type_name, "u64");
    }

    #[test]
    fn test_union() {
        let d = ApiParser::parse_string("union Test { foo: u64, bar: u32 }", "union.def").unwrap();
        assert_eq!(d.unions.len(), 1);
        assert_eq!(d.unions[0].name, "Test");
        assert_eq!(d.unions[0].variables.len(), 2);
        assert_eq!(d.unions[0].variables[0].name, "foo");
        assert_eq!(d.unions[0].variables[0].type_name, "u64");
        assert_eq!(d.unions[0].variables[1].name, "bar");
        assert_eq!(d.unions[0].variables[1].type_name, "u32");
    }

    #[test]
    fn test_const() {
        let def = ApiParser::parse_string("const FOOBAR = \"test\"", "const.def").unwrap();
        assert_eq!(def.consts.len(), 1);
        assert_eq!(def.consts[0].name, "FOOBAR");
        assert_eq!(def.consts[0].value, "\"test\"");
    }

    #[test]
    fn test_const_2() {
        let def = ApiParser::parse_string("const FOOBAR = 0x123", "const.def").unwrap();
        assert_eq!(def.consts.len(), 1);
        assert_eq!(def.consts[0].name, "FOOBAR");
        assert_eq!(def.consts[0].value, "0x123");
    }
}
