
use logos::Logos;
use antimatter::lexer::Token;

fn main() {
    // Test Code
    let code = r#"
abc = 546;
@dsf
#31
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