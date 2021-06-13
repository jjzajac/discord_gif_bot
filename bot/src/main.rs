use std::{collections::HashMap, env, fs, sync::Arc};
use std::borrow::Borrow;
use std::ops::Deref;

use dotenv::dotenv;
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use serenity::utils::MessageBuilder;
use tokio::fs::File;
use tokio::sync::RwLock;

use gif_service::GifService;

struct Handler;

struct Service;

impl TypeMapKey for Service {
    type Value = Arc<RwLock<GifService>>;
}


#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let cmd = msg.content.split(" ").collect::<Vec<&str>>();


        if msg.content.starts_with("::") && msg.content.ends_with("::") {
            let service_lock = {
                let data_read = ctx.data.read().await;

                let service = data_read.get::<Service>().expect("Expected CommandCounter in TypeMap.").clone();
                service
            };

            {
                let service = service_lock.read().await;
                let s = service.clone();
                let e = s.get_url(msg.guild_id.unwrap().to_string(), msg.content[2..msg.content.len() - 2].to_owned()).await.unwrap();

                match msg.channel_id.say(&ctx.http, e.clone()).await {
                    Err(why) => println!("Error sending message: {:?}", why),
                    Ok(t) => println!("Yes{:?}", t),
                }
            };
        } else if cmd[0] == "~help" {
            let message = MessageBuilder::new().push("GifBot cmd:\n\t- ~gif - show all gifs name\n\t- ~send <gif_name> (with attachment) - upload gif and name it with gif_name\n\t- ::gif_name:: - send gif").build();
            match msg.channel_id.say(&ctx.http, message).await {
                Err(why) => println!("Error sending message: {:?}", why),
                Ok(t) => println!("Yes{:?}", t),
            }
        } else if cmd[0] == "~send" {
            if !msg.attachments.is_empty() && msg.attachments[0].filename.split(".").collect::<Vec<&str>>().get(1).unwrap().to_owned() == "gif" {
                println!("{:?}", msg.attachments);
                let service_lock = {
                    let data_read = ctx.data.read().await;

                    let service = data_read.get::<Service>().expect("Expected CommandCounter in TypeMap.").clone();
                    service
                };

                let e = {
                    let service = service_lock.read().await;
                    let s = service.clone();
                    let f = msg.attachments[0].download().await.unwrap();

                    s.upload(msg.guild_id.unwrap().to_string().as_str(), cmd[1], msg.attachments[0].filename.as_str(), f).await;
                };
            }
        } else if cmd[0] == "~gif" {
            let service_lock = {
                let data_read = ctx.data.read().await;

                let service = data_read.get::<Service>().expect("Expected CommandCounter in TypeMap.").clone();
                service
            };

            {
                let service = service_lock.read().await;
                let s = service.clone();

                let v = s.get_name(msg.guild_id.unwrap().to_string()).await.unwrap();

                let mut s = "Gifs:\n".to_owned();
                for vv in v {
                    s += "\t";
                    s += "::";
                    s += vv.as_str();
                    s += "::";
                    s += "\n";
                }
                match msg.channel_id.say(&ctx.http, s).await {
                    Err(why) => println!("Error sending message: {:?}", why),
                    Ok(t) => println!("Yes{:?}", t),
                }
            };
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}


#[tokio::main]
async fn main() {
    dotenv().ok();
    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token in the environment");


    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .await
        .expect("Err creating client");


    {
        let mut data = client.data.write().await;

        let mut gif_service = GifService::new();

        data.insert::<Service>(Arc::new(RwLock::new(gif_service)));
    }


    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
