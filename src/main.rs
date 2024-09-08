
use std::fs::{OpenOptions};
use std::io::{prelude::*, Error};
use genai::chat::printer::print_chat_stream;
use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;
use regex::Regex;
use walkdir::WalkDir;

pub trait Prompt<'a>: Sized {
    fn new(prompt_text: &'a str, user_text: &'a str, model: &'a str, dir: &'a str) -> Self;
    fn write(&self, text: String) -> Result<(), Error>;
    fn build_message(&self) -> ChatRequest;
    fn extracted_text(&self, text: String, file_name: &str) -> Option<String>;
}

struct Text<'a> {
    prompt_text: &'a str,
    user_text: &'a str,
    model: &'a str,
    dir: &'a str,
}

impl<'a> Prompt<'a> for Text<'a> {
    fn new(prompt_text: &'a str, user_text: &'a str, model: &'a str, dir: &'a str) -> Self {
        Self {
            prompt_text,
            user_text,
            model,
            dir
        }
    }

    fn write(&self, text: String) -> Result<(), Error> {
        for entry in WalkDir::new(&self.dir).into_iter() 
        .filter_map(|entry| entry.ok()) {
            let path = entry.path();

            if path.is_file() && path.extension().map_or(false, |ext| ext == "py") {
                
                let mut file = match OpenOptions::new().read(true).write(true).append(true).open(path) {
                    Ok(file) => file,
                    Err(err) => {
                        eprintln!("Error opening file {:?}: {}", path, err);
                        continue;
                    }
                };


                let file_name = path.file_stem().unwrap().to_str().unwrap();

                let item = self.extracted_text(text.clone(), file_name).unwrap();
         

                let mut buffer = String::new();

                file.read_to_string(&mut buffer)?;

    
                if let Err(err) = file.write_all(item.clone().as_bytes()) {
                    eprintln!("Error writing to file {:?}: {}", path, err);
                    continue;
                }    
            }
        }
    
        Ok(())
    }

    fn build_message(&self) -> ChatRequest {
        let chat_req = ChatRequest::new(vec![
            ChatMessage::system(&self.prompt_text.to_owned()),
            ChatMessage::user(&self.user_text.to_owned())
        ]);

        chat_req
    }
    
    fn extracted_text(&self, text: String, file_name: &str) -> Option<String> {
        let normal_text = text.clone();

        let re_start =  Regex::new(&format!(r"(?s)# {}\s*(.*?)\s*(?:# [a-zA-Z0-9_]+\.py|\z)", regex::escape(file_name))).unwrap();

        if let Some(captures) = re_start.captures(&normal_text) {
            if let Some(main_py_code) = captures.get(1) {
                return Some(main_py_code.as_str().to_string());
            }
        } else {
            println!("No match found");
        }

        None
    }

}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example usage
    let client= Client::default();

    let ai_client = Text::new(
        "You are a Python programming language expert, you are very proficient in the language and know its libraries well, and you are also a very good code interpreter, you understand what each code does when you see it and add comments.",
        "main.py
        a = 10 
        print(f'Hello, {a}') 
        
        service.py
        b = 1
        a = b * b * b
        All I want from you is to add comment lines to the codes in the files specific about what the codes do. Just do this and don't write any other text.", 
        "gemini-1.5-flash-latest", 
        "./python");

    let chat_res = client.exec_chat_stream(&ai_client.model, 
        ai_client.build_message(), None).await?;

    let ai_response = print_chat_stream(chat_res, None).await?;

    ai_client.write(ai_response)?;

    Ok(())
}
