//! Service-library instances for the template compilers.
//!
//! A library declares one shared instance exposing its top-level functions; every
//! dependency link lends that instance to its consumer as an argument named after the
//! link, so bodies call `math.maximum(2, 4)` rather than prefixed `math__maximum`.
//! The C17 compiler applies the same contract with a struct of function pointers.
use crate::Node;
use regex::Regex;

pub struct LibraryInstance {
    /// Type of the injected argument in typed languages.
    pub type_symbol: String,
    /// The single shared instance, declared once after the library source.
    pub instance_symbol: String,
    /// Declarations emitted after the library source.
    pub code: String,
}

/// `lib_math` → `Lib_mathLibrary` and `lib_math_library`, as in the C17 compiler.
pub fn symbols(library: &Node) -> (String, String) {
    let base = crate::c17::identifier(&library.name);
    let mut ty = base.clone();
    ty[..1].make_ascii_uppercase();
    (format!("{ty}Library"), format!("{base}_library"))
}

/// Unique top-level names matched by `pattern` (the first participating capture).
fn top_level(source: &str, pattern: &str) -> Vec<String> {
    let mut names = vec![];
    let mut repeated = vec![];
    for capture in Regex::new(pattern).unwrap().captures_iter(source) {
        let Some(name) = capture.iter().skip(1).flatten().next() else {
            continue;
        };
        let name = name.as_str().to_string();
        if names.contains(&name) {
            repeated.push(name);
        } else {
            names.push(name);
        }
    }
    // Overloads cannot be referenced unambiguously as one member.
    names.retain(|n| !repeated.contains(n));
    names
}

/// `module_export` adds a CommonJS export for file-based Node.js modules.
pub fn instance(set: &str, library: &Node, module_export: bool) -> LibraryInstance {
    let (type_symbol, instance_symbol) = symbols(library);
    let source = &library.text.source;
    let instantiation = library
        .text
        .initialization
        .trim()
        .trim_end_matches(';')
        .trim();
    let comment = |prefix: &str| {
        format!(
            "{prefix} Library {} instance, lent to consumers by dependency link name.",
            library.name.replace(['\n', '\r'], " ")
        )
    };
    let code = match set {
        "go" => {
            let pattern =
                Regex::new(r"(?m)^func\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*([^{\n]*?)\s*\{")
                    .unwrap();
            let mut members: Vec<(String, String, String)> = vec![];
            for c in pattern.captures_iter(source) {
                if c[1] != *"main" && c[1] != *"init" && !members.iter().any(|m| m.0 == c[1]) {
                    members.push((c[1].into(), c[2].trim().into(), c[3].trim().into()));
                }
            }
            let fields = members
                .iter()
                .map(|(name, params, results)| {
                    format!(
                        "\t{name} func({params}){}",
                        if results.is_empty() {
                            String::new()
                        } else {
                            format!(" {results}")
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            let values = members
                .iter()
                .map(|(name, _, _)| format!("\t{name}: {name},"))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "{}\ntype {type_symbol} struct {{\n{fields}\n}}\n\nvar {instance_symbol} = &{type_symbol}{{\n{values}\n}}\n",
                comment("//")
            )
        }
        "kotlin" if !instantiation.is_empty() => {
            // Baseline convention: the library declares a class named after itself.
            format!(
                "{}\nval {instance_symbol}: {} = {instantiation}\n",
                comment("//"),
                crate::c17::identifier(&library.name)
            )
        }
        "kotlin" => {
            let names = top_level(
                source,
                r"(?m)^(?:(?:public|internal|inline|suspend|tailrec|operator|infix)\s+)*fun\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(",
            );
            // References are taken at file scope, where a member of the same name cannot shadow them.
            let refs = names
                .iter()
                .map(|n| format!("private val {instance_symbol}__{n} = ::{n}"))
                .collect::<Vec<_>>()
                .join("\n");
            let members = names
                .iter()
                .map(|n| format!("    val {n} = {instance_symbol}__{n}"))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "{}\n{refs}\nobject {type_symbol} {{\n{members}\n}}\nval {instance_symbol} = {type_symbol}\n",
                comment("//")
            )
        }
        "nodejs" | "systemjs" => {
            let value = if instantiation.is_empty() {
                let names = top_level(
                    source,
                    r"(?m)^(?:async\s+)?function\s*\*?\s*([A-Za-z_$][A-Za-z0-9_$]*)\s*\(|^(?:const|let|var)\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*=\s*(?:async\s+)?(?:function\b|\([^)]*\)\s*=>|[A-Za-z_$][A-Za-z0-9_$]*\s*=>)",
                );
                format!("Object.freeze({{ {} }})", names.join(", "))
            } else {
                format!("({instantiation})")
            };
            format!(
                "{}\nconst {instance_symbol} = {value};{}\n",
                comment("//"),
                if module_export {
                    format!("\nmodule.exports.{instance_symbol} = {instance_symbol};")
                } else {
                    String::new()
                }
            )
        }
        "php" => {
            let value = if instantiation.is_empty() {
                let names = top_level(
                    source,
                    r"(?m)^function\s+&?\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(",
                );
                // Methods forward to the file-scope functions; `\name` cannot resolve to the method.
                let methods = names
                    .iter()
                    .map(|n| {
                        format!(
                            "    public function {n}(mixed ...$arguments): mixed\n    {{\n        return \\{n}(...$arguments);\n    }}"
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n");
                format!("new class {{\n{methods}\n}}")
            } else {
                instantiation.to_string()
            };
            format!("{}\n${instance_symbol} = {value};\n", comment("//"))
        }
        "rust" if !instantiation.is_empty() => {
            // As in Kotlin, the library declares a type named after itself.
            format!(
                "{}\ntype {type_symbol} = {};\n\nstatic {instance_symbol}: std::sync::LazyLock<{type_symbol}> =\n    std::sync::LazyLock::new(|| {instantiation});\n",
                comment("//"),
                crate::c17::identifier(&library.name)
            )
        }
        "rust" => {
            // Methods forward to the file-scope functions, which an unqualified call inside
            // the `impl` resolves to. They take the unit instance by value, so elided output
            // lifetimes follow the parameters as in the function itself, and stay private
            // like the library's own items (the program is one module).
            let methods = rust_functions(source)
                .iter()
                .map(|f| {
                    let call = format!("{}({})", f.name, f.arguments.join(", "));
                    format!(
                        "    {}fn {}{}(self{}{}){} {{\n        {}\n    }}",
                        if f.unsafe_ { "unsafe " } else { "" },
                        f.name,
                        f.generics,
                        if f.parameters.is_empty() { "" } else { ", " },
                        f.parameters.join(", "),
                        if f.tail.is_empty() {
                            String::new()
                        } else {
                            format!(" {}", f.tail)
                        },
                        if f.unsafe_ {
                            format!("unsafe {{ {call} }}")
                        } else {
                            call
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            format!(
                "{}\n#[derive(Clone, Copy)]\nstruct {type_symbol};\n\nimpl {type_symbol} {{\n{methods}\n}}\n\nstatic {instance_symbol}: {type_symbol} = {type_symbol};\n",
                comment("//")
            )
        }
        _ => String::new(),
    };
    LibraryInstance {
        type_symbol,
        instance_symbol,
        code,
    }
}

/// A top-level Rust function a library instance forwards to.
struct RustFunction {
    name: String,
    unsafe_: bool,
    /// `<...>` after the name, or empty.
    generics: String,
    /// Parameters, with patterns other than plain names renamed to `argument<i>`.
    parameters: Vec<String>,
    arguments: Vec<String>,
    /// Return type and `where` clause, up to the body.
    tail: String,
}

/// Unique non-async, non-extern functions starting at column 0 (so `impl` members and
/// nested functions are excluded); `main` is not part of a library.
fn rust_functions(source: &str) -> Vec<RustFunction> {
    let start = Regex::new(
        r"(?m)^(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(unsafe\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)",
    )
    .unwrap();
    let mut functions: Vec<RustFunction> = vec![];
    let mut repeated = vec![];
    for capture in start.captures_iter(source) {
        let name = capture[2].to_string();
        let Some(function) = rust_signature(&source[capture.get(0).unwrap().end()..]) else {
            continue;
        };
        if name == "main" {
            continue;
        }
        if functions.iter().any(|f| f.name == name) {
            repeated.push(name);
            continue;
        }
        let (generics, parameters, tail) = function;
        let mut arguments = vec![];
        let parameters = parameters
            .iter()
            .enumerate()
            .map(|(i, parameter)| {
                let (pattern, ty) = split_top_level(parameter, ':')
                    .map(|(p, t)| (p.trim(), t.trim()))
                    .unwrap_or(("", parameter.trim()));
                let pattern = pattern.strip_prefix("mut ").unwrap_or(pattern).trim();
                let plain = !pattern.is_empty()
                    && pattern != "_"
                    && pattern
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_');
                let argument = if plain {
                    pattern.to_string()
                } else {
                    format!("argument{i}")
                };
                arguments.push(argument.clone());
                format!("{argument}: {ty}")
            })
            .collect();
        functions.push(RustFunction {
            name,
            unsafe_: capture.get(1).is_some(),
            generics,
            parameters,
            arguments,
            tail,
        });
    }
    functions.retain(|f| !repeated.contains(&f.name));
    functions
}

/// Generics, parameters and tail of the signature following a function name; `None`
/// when no body follows (a declaration inside `extern`, or unbalanced text).
fn rust_signature(rest: &str) -> Option<(String, Vec<String>, String)> {
    let rest = rest.trim_start();
    let (generics, rest) = if rest.starts_with('<') {
        let end = closing(rest, '<', '>')?;
        (rest[..=end].to_string(), rest[end + 1..].trim_start())
    } else {
        (String::new(), rest)
    };
    if !rest.starts_with('(') {
        return None;
    }
    let end = closing(rest, '(', ')')?;
    let parameters = split_all_top_level(&rest[1..end], ',')
        .into_iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    let after = &rest[end + 1..];
    let body = after.find('{')?;
    if after[..body].contains(';') {
        return None;
    }
    Some((generics, parameters, after[..body].trim().to_string()))
}

/// Index of the delimiter closing the one `text` starts with; `->` is not a closing `>`.
fn closing(text: &str, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    let mut previous = ' ';
    for (i, c) in text.char_indices() {
        if c == open {
            depth += 1;
        } else if c == close && !(close == '>' && previous == '-') {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(i);
            }
        }
        previous = c;
    }
    None
}

/// Splits at every `separator` outside (), [], {} and <> (`->` excepted).
fn split_all_top_level(text: &str, separator: char) -> Vec<&str> {
    let mut parts = vec![];
    let (mut depth, mut start, mut previous) = (0i32, 0, ' ');
    for (i, c) in text.char_indices() {
        match c {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '>' if previous != '-' => depth -= 1,
            _ if c == separator && depth == 0 => {
                // `::` is a path, not a pattern/type separator.
                let path = separator == ':' && (previous == ':' || text[i + 1..].starts_with(':'));
                if !path {
                    parts.push(&text[start..i]);
                    start = i + 1;
                }
            }
            _ => {}
        }
        previous = c;
    }
    parts.push(&text[start..]);
    parts
}

/// The text before and after the first top-level `separator`.
fn split_top_level(text: &str, separator: char) -> Option<(&str, &str)> {
    let first = split_all_top_level(text, separator).first()?.len();
    (first < text.len()).then(|| (&text[..first], &text[first + 1..]))
}
