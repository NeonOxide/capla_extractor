use chumsky::prelude::*;
use chumsky::text::int;
use std::collections::HashSet;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq)]
pub enum CaplaType {
    U64,
    I64,
    F64,
    U8Array { len: String },
    MutU8Array(MutStringLength),
}
#[derive(Debug, Clone, PartialEq)]
pub enum MutStringLength {
    VarName(String),
    FixedNumber(u64),
}
impl Display for MutStringLength {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MutStringLength::VarName(name) => write!(f, "{}", name),
            MutStringLength::FixedNumber(n) => write!(f, "{}", n),
        }
    }
}

impl Display for CaplaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaplaType::U64 => {
                write!(f, "U64")
            }
            CaplaType::I64 => {
                write!(f, "I64")
            }
            CaplaType::F64 => {
                write!(f, "F64")
            }
            CaplaType::U8Array { len } => {
                write!(f, "[u8; {len}]")
            }
            CaplaType::MutU8Array(len) => {
                write!(f, "mut [u8; {len}]")
            }
        }
    }
}
impl CaplaType {
    pub fn as_c_type(&self) -> &'static str {
        match self {
            CaplaType::U64 => "uint64_t",
            CaplaType::I64 => "int64_t",
            CaplaType::U8Array { .. } => "const char*",
            CaplaType::MutU8Array(_) => "char* restrict",
            CaplaType::F64 => "double",
        }
    }

    pub fn as_rocq_type(&self) -> String {
        match self {
            CaplaType::U64 => "native Rocq uint63".to_string(),
            CaplaType::I64 => "native Rocq int63".to_string(),
            CaplaType::MutU8Array(MutStringLength::FixedNumber(n)) => {
                format!("native Rocq string of length {n}")
            }
            CaplaType::U8Array { .. } | CaplaType::MutU8Array(_) => {
                "native Rocq string".to_string()
            }
            CaplaType::F64 => "native Rocq float".to_string(),
        }
    }

    pub fn cast_to_val(&self, val: impl AsRef<str>) -> String {
        match self {
            CaplaType::U64 => format!("Val_long({})", val.as_ref()),
            CaplaType::I64 => format!("Val_long({})", val.as_ref()),
            CaplaType::U8Array { .. } => panic!("U8 arrays should not be return types and as such should not need to be casted to val"),
            CaplaType::MutU8Array(_) => panic!("mutable U8 arrays should not be return types and as such should not need to be casted to val"),
            CaplaType::F64 => format!("mk_float(tinfo, {})", val.as_ref()),
        }
    }

    pub fn cast_from_val(&self, val: impl AsRef<str>) -> String {
        match self {
            CaplaType::U64 => format!("Unsigned_long_val({})", val.as_ref()),
            CaplaType::I64 => format!("Long_val({})", val.as_ref()),
            CaplaType::U8Array { .. } => format!("((char*) {})", val.as_ref()),
            CaplaType::MutU8Array(_) => format!("((char*) {})", val.as_ref()),
            CaplaType::F64 => format!("Double_val({})", val.as_ref()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: CaplaType,
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<CaplaType>,
}

#[derive(Debug, Clone)]
pub enum ValidatedParam {
    RocqParam(Param),
    StringLengthParam {
        name: String,
        string_param_name: String,
    },
    StringOutputParam {
        name: String,
    },
}
impl ValidatedParam {
    pub fn into_capla(&self) -> String {
        match self {
            ValidatedParam::RocqParam(p) => match p.ty {
                CaplaType::U64 => format!("Unsigned_long_val({})", p.name),
                CaplaType::I64 => format!("Long_val({})", p.name),
                CaplaType::U8Array { .. } => format!("((char*) {})", p.name),
                CaplaType::MutU8Array(_) => panic!("Should only be output"),
                CaplaType::F64 => format!("Double_val({})", p.name),
            },
            ValidatedParam::StringLengthParam { name, .. } => name.clone(),
            ValidatedParam::StringOutputParam { name } => name.clone(),
        }
    }
}
#[derive(Debug, Clone)]
pub enum ValidatedReturn {
    /// A numeric return (U64, I64, or F64).
    Numeric(CaplaType),
    /// A string returned via a `mut [u8; ...]` output parameter.
    StringReturn(MutStringLength),
}
impl ValidatedReturn {
    pub fn as_rocq_type(&self) -> String {
        match self {
            ValidatedReturn::Numeric(t) => t.as_rocq_type(),
            ValidatedReturn::StringReturn(s) => match s {
                MutStringLength::VarName(_) => "native Rocq string".to_string(),
                MutStringLength::FixedNumber(n) => {
                    format!("native Rocq string of length {n}")
                }
            },
        }
    }
    pub fn as_c_type(&self) -> &str {
        match self {
            ValidatedReturn::Numeric(t) => t.as_c_type(),
            // It is void because it is passed as a pointer arg
            ValidatedReturn::StringReturn(_) => "void",
        }
    }
    pub fn from_capla(&self, val: impl AsRef<str>) -> String {
        match self {
            ValidatedReturn::Numeric(n) => {
                match n {
                    CaplaType::U64 => format!("Val_long({})", val.as_ref()),
                    CaplaType::I64 => format!("Val_long({})", val.as_ref()),
                    CaplaType::U8Array { .. } => panic!("U8 arrays should not be return types and as such should not need to be casted to val"),
                    CaplaType::MutU8Array(_) => panic!("mutable U8 arrays should not be numeric return types and as such should not need to be casted to val here"),
                    CaplaType::F64 => format!("mk_float(tinfo, {})", val.as_ref()),
                }
            }
            ValidatedReturn::StringReturn(_) => {
                format!("({}, output)", val.as_ref())
            }
        }
    }
}
#[derive(Debug, Clone)]
pub struct ValidatedFunctionSignature {
    pub name: String,
    /// Parameters in source order, classified into their validated roles.
    pub params: Vec<ValidatedParam>,
    /// The original unclassified parameter list, preserved for code generation
    /// (e.g. emitting the C function signature).
    pub original_params: Vec<Param>,
    pub return_type: ValidatedReturn,
}

impl ValidatedFunctionSignature {
    pub fn previous_instructions(&self) -> String {
        let mut instructions = String::new();
        for param in &self.params {
            match param {
                ValidatedParam::RocqParam(_) => {}
                ValidatedParam::StringLengthParam {
                    name,
                    string_param_name,
                } => {
                    instructions.push_str(&format!(
                        "\tuint64_t {name} = prim_strlen({string_param_name});\n"
                    ));
                }
                ValidatedParam::StringOutputParam { .. } => {}
            }
        }
        match &self.return_type {
            ValidatedReturn::Numeric(_) => {}
            ValidatedReturn::StringReturn(s) => match s {
                MutStringLength::VarName(n) => {
                    instructions.push_str(&format!(
                        "\tunsigned char* output = prim_string_make(tinfo, 0, {n});\n"
                    ));
                }
                MutStringLength::FixedNumber(n) => {
                    instructions.push_str(&format!(
                        "\tunsigned char* output = prim_string_make(tinfo, 0, {n});\n"
                    ));
                }
            },
        }
        instructions
    }
}
impl FunctionSignature {
    pub fn returns_string(&self) -> bool {
        self.params
            .iter()
            .any(|p| matches!(p.ty, CaplaType::MutU8Array(_)))
    }

    /// Validate this signature and produce a `ValidatedFunctionSignature`, or
    /// return `None` if it is malformed.
    ///
    /// The rules:
    ///   * Every input string `[u8; L]` must reference a unique parameter `L` of type U64 that
    ///     exists in the signature.
    ///   * Every `mut [u8; L]` with a variable length must likewise reference
    ///     an existing parameter.
    ///   * If the function has a `mut [u8; ...]` parameter (a string output),
    ///     it must NOT also declare a numeric return type. Conversely, if it
    ///     has no string output it MUST declare a numeric return type.
    ///   * At most one `mut [u8; ...]` output parameter is allowed.
    ///
    /// Classification rules for parameters (in source order):
    ///   * A param is `StringLengthParam` iff some **input** string `[u8; L]`
    ///     references it by name. A param referenced only by a `mut [u8; L]`
    ///     output is a normal `RocqParam`.
    ///   * A `mut [u8; ...]` param becomes `StringOutputParam`.
    ///   * Everything else is a `RocqParam`.
    pub fn validate(self) -> Option<ValidatedFunctionSignature> {
        // Every length-by-name reference must resolve to a unique real parameter.
        let mut used_input_length_params = HashSet::new();
        for param in &self.params {
            if let CaplaType::U8Array { len } = &param.ty {
                if !self
                    .params
                    .iter()
                    .any(|p| p.name == *len && p.ty == CaplaType::U64)
                {
                    return None;
                }

                if !used_input_length_params.insert(len) {
                    return None;
                }
            }

            if let CaplaType::MutU8Array(MutStringLength::VarName(len)) = &param.ty {
                if !self
                    .params
                    .iter()
                    .any(|p| p.name == *len && p.ty == CaplaType::U64)
                {
                    return None;
                }
            }
        }

        // Count string outputs and ensure return-type / output-string consistency.
        let mut string_outputs = self
            .params
            .iter()
            .filter(|p| matches!(p.ty, CaplaType::MutU8Array(_)));
        let string_output = string_outputs.next();
        if string_outputs.next().is_some() {
            // More than one mut [u8; ...] parameter is not allowed.
            return None;
        }

        let return_type = match (&self.return_type, string_output) {
            (Some(_), Some(_)) => return None,
            (None, None) => return None,
            (Some(ret), None) => match ret {
                CaplaType::U64 | CaplaType::I64 | CaplaType::F64 => {
                    ValidatedReturn::Numeric(ret.clone())
                }
                // Arrays as a declared return type are not legal.
                CaplaType::U8Array { .. } | CaplaType::MutU8Array(_) => return None,
            },
            (None, Some(out)) => match &out.ty {
                CaplaType::MutU8Array(len) => ValidatedReturn::StringReturn(len.clone()),
                _ => unreachable!("filtered for MutU8Array above"),
            },
        };

        // Build the set of names referenced as the length of an *input* string.
        // Only input-string references promote a param to StringLengthParam;
        // references from mut [u8; ...] outputs do not.
        let input_string_length_refs: Vec<(String, String)> = self
            .params
            .iter()
            .filter_map(|p| {
                if let CaplaType::U8Array { len } = &p.ty {
                    Some((len.clone(), p.name.clone()))
                } else {
                    None
                }
            })
            .collect();

        let validated_params: Vec<ValidatedParam> = self
            .params
            .iter()
            .map(|p| {
                if let Some((_, string_name)) = input_string_length_refs
                    .iter()
                    .find(|(len_name, _)| len_name == &p.name)
                {
                    ValidatedParam::StringLengthParam {
                        name: p.name.clone(),
                        string_param_name: string_name.clone(),
                    }
                } else if matches!(p.ty, CaplaType::MutU8Array(_)) {
                    ValidatedParam::StringOutputParam {
                        name: p.name.clone(),
                    }
                } else {
                    ValidatedParam::RocqParam(p.clone())
                }
            })
            .collect();

        Some(ValidatedFunctionSignature {
            name: self.name.clone(),
            params: validated_params,
            original_params: self.params,
            return_type,
        })
    }
}

fn ident_parser<'a>() -> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> {
    text::unicode::ident().map(str::to_string)
}

fn type_parser<'a>() -> impl Parser<'a, &'a str, CaplaType, extra::Err<Rich<'a, char>>> {
    choice((
        just("u64").ignored().map(|_| CaplaType::U64),
        just("i64").ignored().map(|_| CaplaType::I64),
        just("f64").ignored().map(|_| CaplaType::F64),
        (just("mut").padded().ignore_then(
            just("u8")
                .padded()
                .ignore_then(just(";").padded())
                .ignore_then(
                    text::unicode::ident()
                        .map(|i: &str| {
                            CaplaType::MutU8Array(MutStringLength::VarName(i.to_string()))
                        })
                        .or(int(10).map(|i: &str| {
                            CaplaType::MutU8Array(MutStringLength::FixedNumber(
                                i.parse::<u64>().expect("u64 expected"),
                            ))
                        })),
                )
                .delimited_by(just("["), just("]")),
        )),
        (just("u8")
            .padded()
            .ignore_then(just(";").padded())
            .ignore_then(text::unicode::ident())
            .map(|i: &str| CaplaType::U8Array { len: i.to_string() }))
        .delimited_by(just("["), just("]")),
    ))
}

fn param_group_parser<'a>() -> impl Parser<'a, &'a str, Vec<Param>, extra::Err<Rich<'a, char>>> {
    ident_parser()
        .padded()
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then_ignore(just(":").padded())
        .then(type_parser().padded())
        .map(|(names, ty)| {
            names
                .into_iter()
                .map(|name| Param {
                    name,
                    ty: ty.clone(),
                })
                .collect()
        })
}

fn fun_signature_parser<'a>(
) -> impl Parser<'a, &'a str, FunctionSignature, extra::Err<Rich<'a, char>>> {
    just("fun")
        .ignore_then(ident_parser().padded())
        .then(
            param_group_parser()
                .separated_by(just(",").padded())
                .collect::<Vec<Vec<Param>>>()
                .map(|groups| groups.into_iter().flatten().collect::<Vec<Param>>())
                .delimited_by(just("("), just(")")),
        )
        .then(
            just("->")
                .padded()
                .ignore_then(type_parser().padded())
                .or_not(),
        )
        .map(|((name, params), return_type)| FunctionSignature {
            name,
            params,
            return_type,
        })
}

pub fn parse_b_file(source: &str) -> Vec<ValidatedFunctionSignature> {
    let skip_to_fun = any()
        .and_is(just("fun").then(text::unicode::ident().not()).not())
        .repeated();

    let fun_or_skip = fun_signature_parser()
        .map(Some)
        .or(just("fun").map(|_| None));

    let funs = fun_or_skip
        .padded_by(skip_to_fun)
        .repeated()
        .collect::<Vec<_>>()
        .map(|items| {
            items
                .into_iter()
                .flatten()
                .collect::<Vec<FunctionSignature>>()
        })
        .parse(source)
        .into_result()
        .unwrap_or_else(|err| {
            eprintln!("Failed to parse b file {:?}", err);
            vec![]
        });
    funs.into_iter()
        .filter_map(FunctionSignature::validate)
        .collect()
}
