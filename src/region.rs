use crate::parser::parse_key_value;
use crate::refinements::*;
use crate::opcode_mapping::{Opcode, map_opcode};
use nom::{bytes::complete::tag, character::complete::space0, multi::many0, IResult};
use std::collections::HashMap;
use refinement::Refinement;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Region {
    pub lo_vel: Option<RangeZeroToOneTwentySeven>,
    pub hi_vel: Option<RangeZeroToOneTwentySeven>,
    pub lo_key: Option<RangeZeroToOneTwentySeven>,
    pub hi_key: Option<RangeZeroToOneTwentySeven>,
    pub key: Option<RangeZeroToOneTwentySeven>,
    pub volume: Option<f32>,
    pub region_label: Option<String>,
    pub sample: Option<String>,
    pub offset: Option<u32>,
    pub loop_mode: Option<String>,
    pub trigger: Option<String>,
    pub typed_opcodes: Vec<Opcode>,
    pub parameters: HashMap<String, String>,
}

pub struct SFZFile {
    pub elements: Vec<Region>,
}

pub fn parse_region(sfz_source: &str) -> IResult<&str, Region> {
    let (remaining, _) = tag("<region>")(sfz_source)?;
    parse_region_no_tag(remaining)
}

pub fn parse_region_no_tag(sfz_source: &str) -> IResult<&str, Region> {
    let (remaining, _) = space0(sfz_source)?;
    
    let mut current_input = remaining;
    let mut params = Vec::new();
    
    while !current_input.is_empty() {
        let (rem, _) = nom::character::complete::multispace0(current_input)?;
        if rem.starts_with("<") || rem.starts_with("#") {
            break;
        }
        if let Ok((rem_kv, kv)) = parse_key_value(rem) {
            params.push(kv);
            current_input = rem_kv;
        } else {
            break;
        }
    }

    let mut region = Region::default();
    let mut parameters = HashMap::new();

    for (key, value) in params {
        if let Some(opcode) = map_opcode(key, value) {
            region.typed_opcodes.push(opcode);
        }
        
        match key {
            "lovel" => {
                if let Ok(val) = value.parse::<u8>() {
                    region.lo_vel = RangeZeroToOneTwentySeven::new(val).ok();
                }
            }
            "hivel" => {
                if let Ok(val) = value.parse::<u8>() {
                    region.hi_vel = RangeZeroToOneTwentySeven::new(val).ok();
                }
            }
            "lokey" => {
                if let Ok(val) = value.parse::<u8>() {
                    region.lo_key = RangeZeroToOneTwentySeven::new(val).ok();
                }
            }
            "hikey" => {
                if let Ok(val) = value.parse::<u8>() {
                    region.hi_key = RangeZeroToOneTwentySeven::new(val).ok();
                }
            }
            "key" => {
                if let Ok(val) = value.parse::<u8>() {
                    region.key = RangeZeroToOneTwentySeven::new(val).ok();
                }
            }
            "volume" => {
                region.volume = value.parse::<f32>().ok();
            }
            "label" => {
                region.region_label = Some(value.to_string());
            }
            "sample" => {
                region.sample = Some(value.to_string());
            }
            "offset" => {
                region.offset = value.parse::<u32>().ok();
            }
            "loop_mode" => {
                region.loop_mode = Some(value.to_string());
            }
            "trigger" => {
                region.trigger = Some(value.to_string());
            }
            _ => {
                parameters.insert(key.to_string(), value.to_string());
            }
        }
    }
    region.parameters = parameters;

    Ok((current_input, region))
}

pub fn parse_sfz(sfz_source: &str) -> IResult<&str, SFZFile> {
    let (remaining, elements) = many0(parse_region)(sfz_source)?;
    Ok((remaining, SFZFile { elements }))
}
