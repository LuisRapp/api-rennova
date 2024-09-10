use postgres::{row, Client, NoTls};
use postgres::Error as PostgresError;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::env;

#[macro_use]
extern crate serde_derive;

//Estrucuturas, serian las clases de la base de datos
//Esto es un ejemplo del video
#[derive(Serialize,Deserialize)]
struct User{
    id: Option<i32>,
    name: String,
    email: String,
}

//Conexion a la base de datos
fn obtener_url_basedatos() -> String {
    env::var("URL_BASEDATOS").expect("La variable de entorno URL_BASEDATOS no está configurada")
}



//constantes
const OK_RESPONSE : &str = "HTTP/1.1 200 OK\r\nContent-Type: aplication/json\r\n\r\n";
const NOT_FOUND : &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
const  INTERNAL_SERVER_ERROR: &str= "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n";

//main
fn main(){
    //Set Database
    if let Err(e) = set_database(){
    println!("Error:{}", e);
    return;    
    }
    //start server and print port
    let listener = TcpListener::bind(format!("0.0.0.0:8080")).unwrap();
    println!("Server started at port 8080");
    //handle the client
    for stream in listener.incoming(){
        match stream{
            Ok(stream)=> {
                handle_client(stream);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}



// handle_client function
fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    let mut request = String::new();

    match stream.read(&mut buffer) {
        Ok(size) => {
            request.push_str(String::from_utf8_lossy(&buffer[..size]).as_ref());
            let (status_line, content) = match &*request {
                r if r.starts_with("POST /user") => handle_post_request(r),
                r if r.starts_with("GET /users/") => handle_get_request(r),
                r if r.starts_with("GET /users") => handle_get_all_request(r),
                r if r.starts_with("PUT /users") => handle_put_request(r),
                r if r.starts_with("DELETE /users/") => handle_delete_request(r),
                _ => (NOT_FOUND.to_string(), "Not Found".to_string()),
            };
            stream.write_all(format!("{}{}", status_line, content).as_bytes()).unwrap();
        }
        Err(e) => {
            println!("Error al leer del stream: {}", e); // Manejar el error
        }
    }
}


//Controllers
//handle_post_request function
fn handle_post_request(request: &str)->(String, String){
    match (get_user_request_body(&request), Client::connect(&obtener_url_basedatos(), NoTls)) {
        (Ok(user), Ok(mut client))=> {
            client.execute("INSERT INTO users (name, email) VALUES ($1,$2)", 
            &[&user.name, &user.email]).unwrap();
            (OK_RESPONSE.to_string(),"User created".to_string())
        }
        _=> (INTERNAL_SERVER_ERROR.to_string(),"Error".to_string()),     
    }
}

//handle_get_request function
fn handle_get_request(request: &str) -> (String, String) {
    match (get_id(&request).parse::<i32>(), Client::connect(&obtener_url_basedatos(), NoTls)) {
        (Ok(id), Ok(mut client)) => {
            match client.query("SELECT * FROM user WHERE id = $1", &[&id]) {
                Ok(rows) => {
                    if let Some(row) = rows.get(0) {
                        let user = User {
                            id: row.get(0),
                            name: row.get(1),
                            email: row.get(2),
                        };
                        (OK_RESPONSE.to_string(), serde_json::to_string(&user).unwrap())
                    } else {
                        (NOT_FOUND.to_string(), "User not found".to_string())
                    }
                }
                Err(_) => (INTERNAL_SERVER_ERROR.to_string(), "Error executing query".to_string()),
            }
        }
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

//handle_get_all_request function
fn handle_get_all_request(request: &str)->(String, String){
    match Client::connect(&obtener_url_basedatos(), NoTls) {
        Ok(mut client)=>{
            let mut users = Vec::new();
            for row in client.query("SELECT * FROM users", &[]).unwrap(){
                users.push(User { 
                    id:row.get(0),
                    name:row.get(1),
                    email:row.get(2),
                });
            }
            (OK_RESPONSE.to_string(), serde_json::to_string(&users).unwrap())
        }
        
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}


//handle_put_request function
fn handle_put_request(request: &str)->(String,String){
    match (get_id(&request).parse::<i32>(), 
    get_user_request_body(&request),
    Client::connect(&obtener_url_basedatos(), NoTls)) {
        (Ok(id), Ok(user), Ok(mut client))=>{
            client.execute(
                "UPDATE user SET name=$1, email=$2 where id=$3", &[&user.name, &user.email, &id]
            ).unwrap();
            (OK_RESPONSE.to_string(), "User Updated".to_string())

        }
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

//handle_delete_request function
fn handle_delete_request(request: &str) -> (String, String) {
    match (get_id(&request).parse::<i32>(), Client::connect(&obtener_url_basedatos(), NoTls)) {
        (Ok(id), Ok(mut client)) => {
            match client.execute("DELETE FROM users WHERE id = $1", &[&id]) {
                Ok(row_affected) => {
                    if row_affected == 0 {
                        return (NOT_FOUND.to_string(), "User not found".to_string());
                    }
                    (OK_RESPONSE.to_string(), "User Deleted".to_string())
                }
                Err(_) => (INTERNAL_SERVER_ERROR.to_string(), "Error deleting user".to_string()),
            }
        }
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error connecting to database or parsing ID".to_string()),
    }
}












fn set_database() -> Result<(), PostgresError> {
    // Conexión con la base de datos
    let url_basedatos = obtener_url_basedatos();
    let mut client = Client::connect(&url_basedatos, NoTls)?;
    
    // Crear la tabla
    client.execute(
        "CREATE TABLE IF NOT EXISTS user(
        id SERIAL PRIMARY KEY,
        name VARCHAR NOT NULL,
        email VARCHAR NOT NULL)", &[]
    )?;
    
    Ok(())
}

//get_id function
fn get_id(request: &str) -> &str{
    request.split("/").nth(2).unwrap_or_default().split_whitespace().next().unwrap_or_default()
}

//deserialize user from request body with the id
fn get_user_request_body(request: &str)-> Result<User, serde_json::Error>{
    serde_json::from_str(request.split("\r\n\r\n").last().unwrap_or_default())
}



   

