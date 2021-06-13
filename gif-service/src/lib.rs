use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::time::SystemTime;

use dotenv::dotenv;
use rusoto_core::{Region, RusotoError};
use rusoto_dynamodb::{AttributeValue, DynamoDb, DynamoDbClient, GetItemError, GetItemInput, PutItemInput, PutItemOutput, ScanInput, UpdateItemInput};
use rusoto_s3::{PutObjectError, PutObjectRequest, S3, S3Client};
use uuid::Uuid;

#[derive(Clone)]
pub struct GifService {
    client_dynamo_db: DynamoDbClient,
    table_name: String,
    client_c3: S3Client,
    bucket_name: String,
    bucket_url: String
}

impl GifService {
    pub fn new() -> GifService {
        GifService {
            client_dynamo_db: DynamoDbClient::new(Region::EuWest1),
            table_name: "rust-discord-gif-bot".to_owned(),
            client_c3: S3Client::new(Region::EuWest1),
            bucket_name: "rust-discord-gif-bot".to_string(),
            bucket_url: "https://rust-discord-gif-bot.s3-eu-west-1.amazonaws.com/".to_string()
        }
    }

    pub async fn upload(self, server: &str, gif_name: &str, filename: &str, file: Vec<u8>) -> Result<(), ()> {
        let u = Uuid::new_v3(&Uuid::NAMESPACE_URL, filename.as_bytes());
        let key = format!("{}/{}-{:?}.gif", server.clone(), u.to_string(), SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs());
        println!("{:?}", key);

        let obj = PutObjectRequest {
            bucket: self.bucket_name.clone(),
            body: Some(file.into()),
            content_encoding: Some("image/gif".to_string()),
            content_type: Some("image/gif".to_string()),
            key: key.clone(),
            ..Default::default()
        };

        return match self.client_c3.put_object(obj).await {
            Ok(i) => {
                let mut update_key: HashMap<String, AttributeValue> = HashMap::new();
                update_key.insert("server".to_owned(), AttributeValue {
                    s: Some(server.to_owned()),
                    ..Default::default()
                });

                let mut scan_val: HashMap<String, AttributeValue> = HashMap::new();
                scan_val.insert(":server".to_owned(), AttributeValue {
                    s: Some(server.to_owned()),
                    ..Default::default()
                });
                println!("{:?}", scan_val);
                let scan = ScanInput {
                    expression_attribute_values: Some(scan_val),
                    filter_expression: Some("server = :server".to_owned()),
                    table_name: self.table_name.clone(),
                    ..Default::default()
                };

                match self.client_dynamo_db.scan(scan).await {
                    Ok(x) => {
                        if x.count.unwrap() == 1 {
                            let mut attribute_names: HashMap<String, String> = HashMap::new();
                            attribute_names.insert("#name".to_owned(), gif_name.to_owned());
                            println!("{:?}", attribute_names);
                            let mut attribute_values: HashMap<String, AttributeValue> = HashMap::new();
                            attribute_values.insert(":name".to_owned(), AttributeValue {
                                s: Some(format!("{}{}", self.bucket_url, key)),
                                ..Default::default()
                            });
                            println!("{:?}", attribute_values);


                            let inp = UpdateItemInput {
                                table_name: self.table_name.clone(),
                                key: update_key,
                                return_values: Some("ALL_NEW".to_owned()),
                                expression_attribute_names: Some(attribute_names),
                                expression_attribute_values: Some(attribute_values),
                                update_expression: Some("SET gifs.#name = :name".to_owned()),
                                ..Default::default()
                            };
                            return match self.client_dynamo_db.update_item(inp).await {
                                Ok(i) => {
                                    println!("{:?}", i);


                                    Ok(())
                                }
                                Err(e) => {
                                    // Err(e)
                                    println!("{:?}", e);
                                    Err(())
                                }
                            }
                        } else {
                            let mut val: HashMap<String, AttributeValue> = HashMap::new();
                            val.insert(gif_name.to_owned(), AttributeValue {
                                s: Some(format!("{}{}", self.bucket_url, key)),
                                ..Default::default()
                            });
                            update_key.insert("gifs".to_owned(), AttributeValue {
                                m: Some(val),
                                ..Default::default()
                            });

                            let inp = PutItemInput {
                                table_name: self.table_name.clone(),
                                item: update_key,
                                ..Default::default()
                            };
                            return match self.client_dynamo_db.put_item(inp).await {
                                Ok(i) => {
                                    println!("{:?}", i);


                                    Ok(())
                                }
                                Err(e) => {
                                    // Err(e)
                                    println!("{:?}", e);
                                    Err(())
                                }
                            }
                        }
                    }
                    Err(_e) => {
                        println!("{}", _e);
                        Err(())
                    }
                }
                // Err(e)
            }
            Err(e) => {
                Err(())
            }
        }
    }

    pub async fn get_name(self, server: String) -> Result<Vec::<String>, RusotoError<GetItemError>> {
        let mut key: HashMap<String, AttributeValue> = HashMap::new();
        key.insert("server".to_owned(), AttributeValue {
            s: Some(server),
            ..Default::default()
        });

        let item = GetItemInput {
            table_name: self.table_name,
            key: key,
            projection_expression: Some("gifs".to_owned()),
            ..Default::default()
        };
        return match self.client_dynamo_db.get_item(item).await {
            Ok(i) => {
                println!("{:?}", i);
                let v = i.clone().item.unwrap().get("gifs").unwrap().m.as_ref().unwrap().keys().map(|k| k.clone()).collect::<Vec<String>>();
                Ok(v)
            }
            Err(e) => {
                Err(e)
                // Err(())
            }
        }
    }

    pub async fn get_url(self, server: String, gif_name: String) -> Result<String, RusotoError<GetItemError>> {
        let mut key: HashMap<String, AttributeValue> = HashMap::new();
        key.insert("server".to_owned(), AttributeValue {
            s: Some(server),
            ..Default::default()
        });

        let item = GetItemInput {
            table_name: self.table_name,
            key: key,
            projection_expression: Some("gifs".to_owned()),
            ..Default::default()
        };
        return match self.client_dynamo_db.get_item(item).await {
            Ok(i) => {
                println!("{:?}", i);
                let jj = {
                    i.clone().item.unwrap().get("gifs").unwrap().m.as_ref().unwrap().get(gif_name.as_str()).unwrap().clone().s.unwrap()
                };
                Ok(jj)
            }
            Err(e) => {
                Err(e)
            }
        }
    }
}

