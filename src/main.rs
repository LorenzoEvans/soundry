use soundry::parse_sfz;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sfz = r#"
<control>
default_path=Samples/
#define $EXT wav

<global>
volume=-3

<group>
lovel=0
hivel=127

<region>
sample=kick.$EXT
key=36
"#;

    let document = parse_sfz(sfz)?;
    println!("parsed {} resolved region(s)", document.regions.len());
    Ok(())
}
