use chumsky::prelude::*;
#[derive(Debug, Clone, PartialEq)]
pub enum CaplaType {
    U64,
    F64,
}

impl CaplaType {
    pub fn as_c_type(&self) -> &'static str {
        match self {
            CaplaType::U64 => "uint64_t",
            CaplaType::F64 => "double",
        }
    }

    pub fn cast_to_val(&self, val: impl AsRef<str>) -> String {
        match self {
            CaplaType::U64 => format!("Val_long({})", val.as_ref()),
            CaplaType::F64 => format!("mk_float(tinfo, {})", val.as_ref()),
        }
    }

    pub fn cast_from_val(&self) -> &'static str {
        match self {
            CaplaType::U64 => "Unsigned_long_val",
            CaplaType::F64 => "Double_val",
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
    pub return_type: CaplaType,
}

fn ident_parser<'a>() -> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> {
    text::unicode::ident().map(str::to_string)
}

fn type_parser<'a>() -> impl Parser<'a, &'a str, CaplaType, extra::Err<Rich<'a, char>>> {
    choice((
        just("u64").ignored().map(|_| CaplaType::U64),
        just("f64").ignored().map(|_| CaplaType::F64),
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
                .map(|name| Param { name, ty: ty.clone() })
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
        .then_ignore(just("->").padded())
        .then(type_parser().padded())
        .map(|((name, params), return_type)| FunctionSignature {
            name,
            params,
            return_type,
        })
}

pub fn parse_b_file(source: &str) -> Vec<FunctionSignature> {
    let skip_to_fun = any()
        .and_is(just("fun").then(text::unicode::ident().not()).not())
        .repeated();

    let fun_or_skip = fun_signature_parser()
        .map(Some)
        .or(just("fun").map(|_| None));

    fun_or_skip
        .padded_by(skip_to_fun)
        .repeated()
        .collect::<Vec<_>>()
        .map(|items| items.into_iter().flatten().collect())
        .parse(source)
        .into_result()
        .unwrap_or_else(|err| {
            eprintln!("Failed to parse b file {:?}", err);
            vec![]
        })
}