use crate::norminette_msg::NorminetteMsg;
use nom::bytes::complete::is_not;
use nom::character::complete::{alpha1, anychar, one_of, space1};
use nom::{branch::alt, bytes::complete::tag, character::complete::{i32, newline}, combinator::eof, multi::{many1, many_till}, IResult, Parser};

pub fn parse_norminette(s: &str) -> IResult<&str, Vec<NorminetteMsg>> {
    let (s, _file_name) = is_not(":")(s)?; // line of the form "filename: Error!" or "filename: OK!"
    let (s, _) = tag(": ")(s)?;
    let (s, ok_or_error): (&str, &str) = alt((tag("OK"), tag("Error")))(s)?;
    let (s, _) = tag("!")(s)?;
    let (s, _) = newline(s)?;

    if ok_or_error == "OK" { Ok((s, vec![NorminetteMsg::Ok])) } else { many1(location)(s) }
}

fn location(s: &str) -> IResult<&str, NorminetteMsg> {
    let (s, _) = tag("Error:")(s)?;
    let (s, _) = many1(space1)(s)?;
    let (s, (error_type, _)): (&str, (Vec<&str>, char)) = many_till(alt((alpha1, tag("_"))), one_of(" \t"))(s)?;
    let (s, _) = many1(space1)(s)?;
    let (s, _) = tag("(line:")(s)?;
    let (s, _) = many1(space1)(s)?;
    let (s, l) = i32(s)?;
    let (s, _) = tag(", col:")(s)?;
    let (s, _) = many1(space1)(s)?;
    let (s, c) = i32(s)?;
    let (s, _) = tag("):")(s)?;
    let (s, _) = many1(space1)(s)?;
    let (s, (msg, _)): (&str, (Vec<char>, &str)) = many_till(anychar, alt((newline.map(|_| ""), eof)))(s)?;

    Ok((
        s,
        NorminetteMsg::Error {
            error_type: error_type.into_iter().collect(),
            line: l,
            column: c,
            message: msg.into_iter().collect(),
        },
    ))
}
