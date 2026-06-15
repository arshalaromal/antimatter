
use logos::Logos;
use antimatter::lexer::Token;

fn main() {
    // Test Code
    let code = r#"
        use std::net;

        type Port = Int;
        struct Server {
            address: String;
            port: Port; // Semantic meaning is now clear
        }

        fn main() {
            /* 1. Setup the server
               2. Using the alias
            */
            const myPort: Port = 8080;

            var s = Server {
                address: "127.0.0.1",
                port: myPort
            };

            println("Server running on port: " + s.port);
        }
    "#;

    println!("Source Code:\n{}", code);
    println!("-----------------------------\nTokens:");

    let mut lexer = Token::lexer(code);


    // Loop through and print every token
    while let Some(result) = lexer.next() {
        match result {
            Ok(token) => println!("{:?}", token),
            Err(_) => {
                println!("LEXER ERROR: Unrecognized token at '{}'", lexer.slice());
            }
        }
    }
}