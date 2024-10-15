use anyhow::Context as _;
use poise::serenity_prelude::model::Colour;
use poise::serenity_prelude::{ClientBuilder, GatewayIntents, CreateEmbed};
use poise::reply::CreateReply;
use shuttle_runtime::SecretStore;
use shuttle_serenity::ShuttleSerenity;
use regex::Regex;
use std::fs;
use std::io::Read;

struct Data {
    cr : serde_json::Value // the comprehensive rules json
} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;


// for now use the default help function : prettier & more complete TODO
/// Show the help menu
#[poise::command(prefix_command, slash_command, track_edits)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            extra_text_at_bottom: "Inventor of hieroglyphic writing and patron of scribes in Egyptian mythology, Thoth symbolizes wisdom. He is the holder of all the world's knowledge, including science, mathematics and architecture. And in this role, the god with the head of an ibis – or sometimes a baboon – is also responsible for sharing and disseminating this knowledge around the world. Because of his impartiality, Thoth was also given the role of presiding over the tribunal of Osiris – which judges the dead – alongside Anubis.",
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

/// gives an access path from a rule number
/// eg. "1.2.a" ~> ["0", "1", "CONTENT", "1.2", "CONTENT", "1.2.a"]
fn access_path(number:&String) -> Vec<String> {
    let v : Vec<&str> = number.split(".").collect() ;
    let mut access= vec!["RULES".to_string(), v[0].to_string()] ;
    for i in 1..v.len() {
        access.push("CONTENT".to_string()) ;
        let mut index = v[0].to_string() ;
        for j in 1..(i+1) {
            index = index + "." + v[j] ;
        }
        access.push(index) ;
    }
    access
}

/// builds a JSON pointer to access a rule from its number
/// eg. "1.2.a" ~> "0/1/CONTENT/1.2/CONTENT/1.2.a
fn build_pointer(number:&String) -> String {
    let access = access_path(number) ;
    let mut pointer = String::new() ;
    for a in access {
        pointer = pointer + "/" + a.as_str() ;
    }
    println!("{}", pointer) ;
    pointer
}

/// list of the names of sections containing the specified rule
fn sections(number:&String, mut head: &serde_json::Value) -> String {
    let access = access_path(number) ;
    let mut list_sections = vec![] ;

    for i in 0..access.len() {
        head = head.get(&access[i])
            .expect("Invalid access path to sections") ;
        match head.get("NAME") {
            Some(name) => list_sections.push(name.as_str().unwrap()) ,
            None => ()
        }
    }

    let mut sections = list_sections[0].to_string() ;
    for i in 1..list_sections.len() {
        sections = sections + " / " + list_sections[i];
    }
    sections
}

/// replaces the icon names with discord emojis
fn iconify(mut text : String) -> String {
    let icon_id = vec![
        ("1", "<:icon_1:1294668469257633874>"),
        ("2", "<:icon_2:1294668512035344384>"),
        ("3", "<:icon_3:1294668541651324969>"),
        ("4", "<:icon_4:1294668575738691687>"),
        ("5", "<:icon_5:1294668611083960361>"),
        ("6", "<:icon_6:1294668649621094554>"),
        ("7", "<:icon_7:1294668688347369564>"),
        ("8", "<:icon_8:1294668756521324646>"),
        ("9", "<:icon_9:1294668790566617088>"),
        ("x", "<:icon_x:1294668825354309643>"),
        ("j", "<:icon_j:1294649576124321813>"),
        ("r", "<:icon_r:1294668318812143697>"),
        ("h", "<:icon_h:1294670002439454775>"),
        ("T", "<:icon_T:1294670251493032000>"),
        ("D", "<:icon_D:1294670405251891250>"),
        ("O", "<:icon_O:1294671037681897584>"),
        ("V", "<:icon_V:1294671319320891433>"),
        ("M", "<:icon_M:1294671220221939826>"),
        ("common", "<:icon_common:1294678013404774422>"),
        ("rare"  , "<:icon_rare:1294678128664379522>"),
        ("unique", "<:icon_unique:1294678091007918120>"),
        ("axiom" , "<:icon_axiom:1294678780144521330>"),
        ("bravos", "<:icon_bravos:1294678819608727693>"),
        ("lyra"  , "<:icon_lyra:1294678874403110913>"),
        ("muna"  , "<:icon_muna:1294678905541890171>"),
        ("ordis" , "<:icon_ordis:1294678944959823872>"),
        ("yzmir" , "<:icon_yzmir:1294678973246083124>")        
    ] ;
    for (x,id) in icon_id {

        let re = Regex::new(
            format!(r"%{}%", x).as_str()
        ).unwrap() ;
        text = re.replace_all(text.as_str(), id).to_string() ;

    }
    text
}

/// paragraph with section contents
fn section_contents(section : &serde_json::Value) -> String {
    let mut text = "".to_string() ;
    let map_contents = section.as_object().unwrap() ;
    for (k, v) in map_contents.iter() {
        text = text + k.as_str() ;
        match v.get("NAME") {
            Some(n) => text = text + " - " + n.as_str().unwrap() ,
            None => {
                let mut rule = v.get("RULE").unwrap()
                    .as_str().unwrap().to_string() ;
                rule.truncate(40);
                text = text + " *" + rule.as_str() + "...*"
            }
        }
        text = text + "\n" ;
    }
    text
}

/// Displays the requested ruling
#[poise::command(prefix_command, slash_command, aliases("rule"), track_edits)]
async fn cr(ctx: Context<'_>,
    #[description = "Rule to display"] number: String, 
) -> Result<(), Error> {
    let cr = &ctx.data().cr ;
    let item = cr.pointer(build_pointer(&number).as_str()) ;
    let mut embed_msg = CreateEmbed::new().color(Colour::BLUE) ;
    match item {
        None => embed_msg = embed_msg
                    .color(Colour::RED)
                    .title("Invalid rule number")
                    .description("Error - Rule \"".to_string() + number.as_str() + "\" not found") ,
        Some(v) => match v.get("NAME") {
            // is an actual ruling
            None => {
                let map_rule = v.as_object().unwrap() ;
                for (k, vv) in map_rule.iter() {
                    let vv_text = iconify(vv.as_str().unwrap().to_string()) ;
                    if k == "RULE" {
                        embed_msg = embed_msg.description(vv_text)
                    } else {
                        embed_msg = embed_msg.field(k, vv_text, false) 
                    }
                }
                embed_msg = embed_msg
                    .title("Rule ".to_string() + number.as_str())
                    .field("In section", sections(&number, cr), false)
            } ,
            // is a section : display its name
            Some(n) => {
                embed_msg = embed_msg
                    .title("Section ".to_string() + number.as_str() + " - " + n.as_str().unwrap())
                    .description(sections(&number, cr)) 
                    .field("Contents", section_contents(v.get("CONTENT").unwrap()), false)
            }
        }
    }
    let builder = CreateReply::default().embed(embed_msg) ;
    ctx.send(builder).await?;
    Ok(())
}


/// Displays the table of contents
#[poise::command(prefix_command, slash_command, track_edits)]
async fn contents(ctx: Context<'_>) -> Result<(), Error> {
    let cr = &ctx.data().cr ;
    let toc = section_contents(cr.get("RULES").unwrap()) ;
    let about = cr.get("ABOUT").unwrap().as_str().unwrap() ;
    let embed_msg = CreateEmbed::new().color(Colour::BLUE)
        .title("Table of contents")
        .description(toc) 
        .field("About", about, false);
    let builder: CreateReply = CreateReply::default().embed(embed_msg) ;
    ctx.send(builder).await?;
    Ok(())
}


/// Displays the glossary entry of the requested word 
#[poise::command(prefix_command, slash_command, aliases("about"), track_edits)]
async fn glossary(ctx: Context<'_>,
    #[description = "Word to search for in the glossary"] word: String, 
    #[description = "If to display the related rules (default:True)"] related: Option<bool>, 
) -> Result<(), Error> {
    let cr = &ctx.data().cr ;
    let item = cr["GLOSSARY"].get(&word);

    let mut embed_msg = CreateEmbed::new().color(Colour::BLUE) ;

    match item {
        None => embed_msg = embed_msg
                .color(Colour::RED) 
                .title("Not found")
                .description("Error - Entry \"".to_string() + word.as_str() + "\" not found") ,
        Some(v) => {
            let text = v.get("DESCRIPTION").expect("Missing description in glossary").as_str().unwrap().to_string() ;
            embed_msg = embed_msg
                .title(word.as_str())
                .description(text) ;
            match related {
                Some(false) => () ,
                _ => { 
                    let mut text_rel = "".to_string() ;
                    for r in v.get("RELATED").expect("Missing related rules in glossary").as_array().unwrap() {
                        text_rel = text_rel + r.as_str().unwrap() + " " ;
                    }
                    embed_msg = embed_msg.field("Related rules", text_rel, false)
                }
            }
        }

    }
    let builder = CreateReply::default().embed(embed_msg) ;
    ctx.send(builder).await?;
    Ok(())
}

/// Version of the Comprehensive rules
#[poise::command(prefix_command, slash_command)]
async fn version(ctx: Context<'_>) -> Result<(), Error> {
    
    let cr_version = ctx.data().cr["VERSION"].as_str().unwrap() ;

    let mut toml_file = fs::File::open("Cargo.toml")
        .expect("Unable to read `Cargo.toml`") ;
    let mut toml_string = String::new() ;
    toml_file.read_to_string(&mut toml_string)?;
    let re = Regex::new(r#"version = "([\w\.-]*)""#).unwrap() ;
    let bot_version = re.captures(toml_string.as_str())
        .expect("Unable to find version in `Cargo.toml`")
        .get(1).unwrap()
        .as_str();

    let embed_msg = CreateEmbed::new().color(Colour::BLUE)
        .title("Version")
        .field("Comprehensive Rules Version", cr_version, false)
        .field("Bot Version", bot_version, false) ;
    let builder = CreateReply::default().embed(embed_msg) ;
    ctx.send(builder).await?;
    Ok(())

}


#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secret_store: SecretStore) -> ShuttleSerenity {
    // Get the discord token set in `Secrets.toml`
    let discord_token = secret_store
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;

    let cr_file = fs::File::open("cr.json")
        .expect("Unable to read file");

    let cr_json: serde_json::Value = serde_json::from_reader(cr_file)
        .expect("Incorrect JSON file") ;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                cr(),
                glossary(),
                contents(),
                help(),
                version(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("??".into()),
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data { cr : cr_json })
            })
        })
        .build();

    let intents =
        GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let client = ClientBuilder::new(discord_token, intents)
        .framework(framework)
        .await
        .map_err(shuttle_runtime::CustomError::new)?;

    Ok(client.into())
}