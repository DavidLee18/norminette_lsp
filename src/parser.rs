use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{anychar, i32, newline, none_of, space0},
    combinator::eof,
    multi::{many0, many1, many_m_n, many_till},
    IResult, Parser,
};

use crate::norminette_msg::NorminetteMsg;

pub fn parse_norminette(s: &str) -> IResult<&str, Vec<NorminetteMsg>> {
    let (s, _) = many_m_n(0, 1, header)(s)?;
    let (s, _) = many0(none_of("\n"))(s)?; // line of the form "filename: Error!" or "filename: OK!"
    let (s, _) = newline(s)?;
    many0(alt((location, invalid)))(s)
}

fn header(s: &str) -> IResult<&str, ()> {
    let (s, _) = tag("Missing or invalid header")(s)?;
    let (s, _) = many0(none_of("\n"))(s)?;
    let (s, _) = newline(s)?;
    Ok((s, ()))
}

fn invalid(s: &str) -> IResult<&str, NorminetteMsg> {
    let (s, _) = many1(none_of(" "))(s)?;
    let (s, _) = many0(anychar)(s)?;
    Ok((
        s,
        NorminetteMsg::NoLocation {
            message: format!("file{}", s),
        },
    ))
}

fn location(s: &str) -> IResult<&str, NorminetteMsg> {
    let (s, _) = tag("Error:")(s)?;
    let (s, _) = space0(s)?;
    let (s, error_type) = many0(none_of("\n"))(s)?;
    let (s, _) = space0(s)?;
    let (s, _) = tag("(line: ")(s)?;
    let (s, _) = space0(s)?;
    let (s, l) = i32(s)?;
    let (s, _) = tag(", col:")(s)?;
    let (s, _) = space0(s)?;
    let (s, c) = i32(s)?;
    let (s, _) = tag("):\t")(s)?;
    let (s, _) = space0(s)?;
    let (s, (msg, _)) = many_till(anychar, alt((newline.map(|_| ""), eof)))(s)?;

    Ok((
        s,
        NorminetteMsg::LineColumn {
            error_type: error_type.into_iter().collect(),
            line: l,
            column: c,
            message: msg.into_iter().collect(),
        },
    ))
}
