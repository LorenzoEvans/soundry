use crate::header_types::Control;
use crate::parser::{white_space, take_to_newline, parse_identifier, parse_key_value, parse_value};
use nom::{
    sequence::tuple,
    IResult,
    multi::{many0, many1},
    bytes::complete::{tag, take_while, take_while1},
    character::complete::{newline, space1, char, multispace0},
};

use std::path::PathBuf;

fn is_alphanumeric_underscore(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn variable_name(sfz_source: &str) -> IResult<&str, &str> {
    let (remaining, _) = tag("$")(sfz_source)?;
    take_while1(|c: char| c.is_alphanumeric() || c == '_')(remaining)
}

fn variable_value(sfz_source: &str) -> IResult<&str, &str> {
    take_while1(|c: char| !c.is_whitespace())(sfz_source)
}

fn parse_define_line(sfz_source: &str) -> IResult<&str, (&str, &str)> {
    let (remaining, _) = multispace0(sfz_source)?;
    let (remaining, _) = tag("#define")(remaining)?;
    let (remaining, _) = space1(remaining)?;
    let (remaining, var_name) = variable_name(remaining)?;
    let (remaining, _) = space1(remaining)?;
    let (remaining, var_value) = variable_value(remaining)?;
    Ok((remaining, (var_name, var_value)))
}

fn parse_defines(sfz_source: &str) -> IResult<&str, Vec<(&str, &str)>> {
    many1(parse_define_line)(sfz_source)
}

pub fn parse_default_path(sfz_source: &str) -> IResult<&str, &str> {
    let (remaining, (key, value)) = parse_key_value(sfz_source)?;
    if key == "default_path" {
        Ok((remaining, value))
    } else {
        Err(nom::Err::Error(nom::error::Error::new(sfz_source, nom::error::ErrorKind::Tag)))
    }
}

pub fn add_define_directives(control_header: &mut Control, directives: Vec<(&str, &str)>) {
    for (define_var, define_value) in directives {
        control_header
            .define_directives
            .insert(format!("${}", define_var), define_value.to_owned());
    }
}

fn dequote(sfz_source: &str) -> IResult<&str, &str> {
    let (remaining, _) = char('"')(sfz_source)?;
    let (remaining, include_directive) = take_until1("\"")(remaining)?;
    let (remaining, _) = char('"')(remaining)?;
    Ok((remaining, include_directive))
}

pub fn parse_include_line(sfz_source: &str) -> IResult<&str, &str> {
    let (remaining, _) = multispace0(sfz_source)?;
    let (remaining, _) = tag("#include")(remaining)?;
    let (remaining, _) = space1(remaining)?;
    let (remaining, include_directive) = dequote(remaining)?;
    Ok((remaining, include_directive))
}

fn parse_includes(sfz_source: &str) -> IResult<&str, Vec<&str>> {
    many0(parse_include_line)(sfz_source)
}

pub fn parse_control(sfz_source: &str) -> IResult<&str, Control> {
    let mut control_header = Control::new();
    let (remaining, header_tag) = crate::parser::parse_header_tag(sfz_source)?;
    if header_tag != "control" {
         return Err(nom::Err::Error(nom::error::Error::new(sfz_source, nom::error::ErrorKind::Tag)));
    }

    let mut current_input = remaining;

    // We need to parse items in any order inside <control>
    loop {
        if let Ok((rem, default_path)) = parse_default_path(current_input) {
            control_header.default_path = PathBuf::from(default_path);
            current_input = rem;
            continue;
        }
        if let Ok((rem, (var_name, var_value))) = parse_define_line(current_input) {
            control_header.define_directives.insert(format!("${}", var_name), var_value.to_owned());
            current_input = rem;
            continue;
        }
        if let Ok((rem, include_path)) = parse_include_line(current_input) {
            control_header.include_directives.push(include_path.to_string());
            current_input = rem;
            continue;
        }
        if let Ok((rem, (label_cc, label_number, label_value))) = parse_cc_label(current_input) {
            add_label_ccns(&mut control_header, vec![(label_cc, label_number, label_value)]);
            current_input = rem;
            continue;
        }
        if let Ok((rem, (set_number, set_value))) = parse_set_ccn(current_input) {
            control_header.set_ccn.insert(set_number.to_string(), set_value.to_string());
            current_input = rem;
            continue;
        }
        
        break;
    }

    Ok((current_input, control_header))
}

pub fn add_include_directives(control_header: &mut Control, directives: Vec<&str>) {
    for i in directives {
        control_header.include_directives.push(i.to_string());
    }
}

pub fn add_label_ccns(control_header: &mut Control, label_ccns: Vec<(&str, &str, &str)>) {
    for cc_tuple in label_ccns {
        let (_label, label_number, label_value) = cc_tuple;
        let cc_label = format!("label_cc{}", label_number);

        if label_value.contains(" ") {
            let mut label_value_iter = label_value.split_whitespace();
            let instrument = label_value_iter.next().unwrap().to_owned();
            let modulation = label_value_iter.next().map(|s| s.to_owned());
            control_header.label_ccn.insert(cc_label, (instrument, modulation));
        } else {
            control_header.label_ccn.insert(cc_label, (label_value.to_owned(), None));
        }
    }
}

pub fn parse_cc_label(sfz_source: &str) -> IResult<&str, (&str, &str, &str)> {
    let (remaining, _) = multispace0(sfz_source)?;
    let (remaining, key) = parse_identifier(remaining)?;
    
    if !key.starts_with("label_cc") {
        return Err(nom::Err::Error(nom::error::Error::new(sfz_source, nom::error::ErrorKind::Tag)));
    }
    
    let label_number = &key[8..];
    let (remaining, _) = tag("=")(remaining)?;
    let (remaining, label_value) = take_to_newline(remaining)?;

    Ok((remaining, ("label_cc", label_number, label_value)))
}

pub fn parse_cc_var(sfz_source: &str) -> IResult<&str, (&str, &str), > {
    let (remaining, (var_name, _,  var_value)) = tuple((parse_identifier, tag("="), take_to_newline))(sfz_source)?;
    Ok((remaining, (var_name, var_value)))
}

pub fn parse_set_ccn(sfz_source: &str) -> IResult<&str, (&str, &str)> {
    let (remaining, _) = multispace0(sfz_source)?;
    let (remaining, key) = parse_identifier(remaining)?;
    if !key.starts_with("set_cc") {
        return Err(nom::Err::Error(nom::error::Error::new(sfz_source, nom::error::ErrorKind::Tag)));
    }
    let (remaining, _) = tag("=")(remaining)?;
    let (remaining, value) = parse_value(remaining)?;
    Ok((remaining, (key, value)))
}
