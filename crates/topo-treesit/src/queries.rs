//! Inline S-expression queries for tree-sitter grammars.
//!
//! Each language gets a query string with standardized capture names:
//! - `@function` — function/method outer node
//! - `@type` — struct/class/enum/trait/interface outer node
//! - `@impl` — impl block outer node
//! - `@import` — use/import statement outer node
//! - `@name` — identifier node inside the above (for symbol name extraction)

use topo_core::Language;

pub fn query_for(language: Language) -> Option<&'static str> {
    match language {
        Language::Rust => Some(RUST),
        Language::Go => Some(GO),
        Language::Python => Some(PYTHON),
        Language::JavaScript => Some(JAVASCRIPT),
        Language::TypeScript => Some(TYPESCRIPT),
        Language::Java => Some(JAVA),
        Language::Ruby => Some(RUBY),
        Language::C => Some(C),
        Language::Cpp => Some(CPP),
        Language::Shell => Some(BASH),
        Language::Swift => Some(SWIFT),
        Language::Kotlin => Some(KOTLIN),
        Language::Scala => Some(SCALA),
        Language::Haskell => Some(HASKELL),
        Language::Elixir => Some(ELIXIR),
        Language::Lua => Some(LUA),
        Language::Php => Some(PHP),
        Language::R => Some(R),
        // Data/markup languages — no meaningful code chunks
        Language::Markdown
        | Language::Yaml
        | Language::Toml
        | Language::Json
        | Language::Html
        | Language::Css
        | Language::Other => None,
    }
}

const RUST: &str = r#"
(function_item name: (identifier) @name) @function
(struct_item name: (type_identifier) @name) @type
(enum_item name: (type_identifier) @name) @type
(trait_item name: (type_identifier) @name) @type
(type_item name: (type_identifier) @name) @type
(impl_item) @impl
(use_declaration) @import
"#;

const GO: &str = r#"
(function_declaration name: (identifier) @name) @function
(method_declaration name: (field_identifier) @name) @function
(type_declaration (type_spec name: (type_identifier) @name)) @type
(import_declaration) @import
"#;

const PYTHON: &str = r#"
(function_definition name: (identifier) @name) @function
(class_definition name: (identifier) @name) @type
(import_statement) @import
(import_from_statement) @import
"#;

const JAVASCRIPT: &str = r#"
(function_declaration name: (identifier) @name) @function
(class_declaration name: (identifier) @name) @type
(method_definition name: (property_identifier) @name) @function
(import_statement) @import
"#;

const TYPESCRIPT: &str = r#"
(function_declaration name: (identifier) @name) @function
(class_declaration name: (type_identifier) @name) @type
(method_definition name: (property_identifier) @name) @function
(interface_declaration name: (type_identifier) @name) @type
(type_alias_declaration name: (type_identifier) @name) @type
(enum_declaration name: (identifier) @name) @type
(import_statement) @import
"#;

const JAVA: &str = r#"
(class_declaration name: (identifier) @name) @type
(interface_declaration name: (identifier) @name) @type
(enum_declaration name: (identifier) @name) @type
(method_declaration name: (identifier) @name) @function
(constructor_declaration name: (identifier) @name) @function
(import_declaration) @import
"#;

const RUBY: &str = r#"
(method name: (identifier) @name) @function
(singleton_method name: (identifier) @name) @function
(class name: (constant) @name) @type
(module name: (constant) @name) @type
(call method: (identifier) @name) @import
"#;

const C: &str = r#"
(function_definition declarator: (function_declarator declarator: (identifier) @name)) @function
(struct_specifier name: (type_identifier) @name) @type
(enum_specifier name: (type_identifier) @name) @type
(union_specifier name: (type_identifier) @name) @type
(type_definition declarator: (type_identifier) @name) @type
(preproc_include) @import
"#;

const CPP: &str = r#"
(function_definition declarator: (function_declarator declarator: (identifier) @name)) @function
(function_definition declarator: (function_declarator declarator: (qualified_identifier) @name)) @function
(class_specifier name: (type_identifier) @name) @type
(struct_specifier name: (type_identifier) @name) @type
(enum_specifier name: (type_identifier) @name) @type
(namespace_definition name: (identifier) @name) @type
(preproc_include) @import
"#;

const BASH: &str = r#"
(function_definition name: (word) @name) @function
"#;

const SWIFT: &str = r#"
(function_declaration name: (simple_identifier) @name) @function
(class_declaration name: (type_identifier) @name) @type
(struct_declaration name: (type_identifier) @name) @type
(enum_declaration name: (type_identifier) @name) @type
(protocol_declaration name: (type_identifier) @name) @type
(import_declaration) @import
"#;

const KOTLIN: &str = r#"
(function_declaration (simple_identifier) @name) @function
(class_declaration (type_identifier) @name) @type
(object_declaration (type_identifier) @name) @type
(import_header) @import
"#;

const SCALA: &str = r#"
(function_definition name: (identifier) @name) @function
(class_definition name: (identifier) @name) @type
(trait_definition name: (identifier) @name) @type
(object_definition name: (identifier) @name) @type
(import_declaration) @import
"#;

const HASKELL: &str = r#"
(function name: (variable) @name) @function
(signature name: (variable) @name) @function
(type_alias name: (type) @name) @type
(newtype name: (type) @name) @type
(adt name: (type) @name) @type
(class name: (type) @name) @type
(import) @import
"#;

const ELIXIR: &str = r#"
(call target: (identifier) @_keyword arguments: (arguments (identifier) @name)) @function
"#;

const LUA: &str = r#"
(function_declaration name: (identifier) @name) @function
(function_declaration name: (dot_index_expression) @name) @function
"#;

const PHP: &str = r#"
(function_definition name: (name) @name) @function
(method_declaration name: (name) @name) @function
(class_declaration name: (name) @name) @type
(interface_declaration name: (name) @name) @type
(trait_declaration name: (name) @name) @type
(enum_declaration name: (name) @name) @type
(namespace_use_declaration) @import
"#;

const R: &str = r#"
(function_definition name: (identifier) @name) @function
(left_assignment name: (identifier) @name) @function
"#;
