# api-rennova: Documentacion del Codigo  
## Documentacion de las funciones  
Aquí tienes una documentación resumida de todo el código:

- **`obtener_url_basedatos`**: Recupera la URL de la base de datos desde una variable de entorno.

- **`set_database`**: Establece la conexión con la base de datos y crea la tabla `user` si no existe.

- **`get_id`**: Extrae el ID del usuario desde la URL de la solicitud.

- **`get_user_request_body`**: Deserializa el cuerpo de una solicitud en formato JSON para obtener un objeto `User`.

- **`handle_client`**: Lee una solicitud HTTP y redirige a la función correspondiente según el método y la ruta (POST, GET, PUT, DELETE).

- **`handle_post_request`**: Inserta un nuevo usuario en la base de datos a partir de los datos enviados en la solicitud.

- **`handle_get_request`**: Busca un usuario por su ID en la base de datos y lo devuelve en formato JSON.

- **`handle_get_all_request`**: Devuelve todos los usuarios de la base de datos en formato JSON.

- **`handle_put_request`**: Actualiza los datos de un usuario existente en la base de datos.

- **`handle_delete_request`**: Elimina un usuario de la base de datos por su ID.

- **`main`**: Inicializa el servidor TCP en el puerto 8080 y maneja las solicitudes entrantes.

----  
```rust
use postgres::{row, Client, NoTls};
use postgres::Error as PostgresError;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::env;

#[macro_use]
extern crate serde_derive;

/// Estructura que representa un usuario en la base de datos.
/// La estructura `User` incluye tres campos:
/// - `id`: un valor opcional que contiene el identificador único del usuario (si ya ha sido creado).
/// - `name
// Constantes de respuesta HTTP
// Estas constantes representan diferentes respuestas HTTP que el servidor enviará
// según el resultado de las solicitudes.
const OK_RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n";
const NOT_FOUND: &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
const INTERNAL_SERVER_ERROR: &str = "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n";

// Función principal `main`
fn main() {
    // Configuración de la base de datos
    // Se establece la conexión con la base de datos llamando a la función `set_database`.
    // Si ocurre un error, se imprime y se detiene la ejecución.
    if let Err(e) = set_database() {
        println!("Error: {}", e);
        return;
    }

    // Inicia el servidor y escucha en el puerto 8080
    let listener = TcpListener::bind(format!("0.0.0.0:8080")).unwrap();
    println!("Server started at port 8080");

    // Maneja las conexiones entrantes
    // El servidor escucha las solicitudes entrantes en un bucle, y para cada conexión,
    // llama a la función `handle_client` para procesarla.
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_client(stream);  // Procesa la solicitud del cliente
            }
            Err(e) => {
                println!("Error: {}", e);  // Maneja los errores de conexión
            }
        }
    }
}

// Función `handle_client` para procesar las solicitudes del cliente
// Esta función recibe una conexión (stream) y lee la solicitud del cliente,
// luego determina el tipo de solicitud (GET, POST, PUT, DELETE) y envía una
// respuesta correspondiente.
fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 1024];  // Buffer para almacenar los datos de la solicitud
    let mut request = String::new();  // Cadena donde se almacenará la solicitud completa

    // Lee los datos del cliente
    match stream.read(&mut buffer) {
        Ok(size) => {
            // Convierte los datos leídos en una cadena y la almacena en `request`
            request.push_str(String::from_utf8_lossy(&buffer[..size]).as_ref());

            // Determina el tipo de solicitud (POST, GET, PUT, DELETE)
            let (status_line, content) = match &*request {
                r if r.starts_with("POST /user") => handle_post_request(r),  // POST
                r if r.starts_with("GET /users/") => handle_get_request(r),  // GET (con ID)
                r if r.starts_with("GET /users") => handle_get_all_request(r),  // GET (todos)
                r if r.starts_with("PUT /users") => handle_put_request(r),  // PUT (actualizar)
                r if r.starts_with("DELETE /users/") => handle_delete_request(r),  // DELETE
                _ => (NOT_FOUND.to_string(), "Not Found".to_string()),  // Si no coincide con ninguna ruta
            };

            // Envía la respuesta al cliente
            stream.write_all(format!("{}{}", status_line, content).as_bytes()).unwrap();
        }
        Err(e) => {
            // Maneja el error en la lectura de la solicitud
            println!("Error al leer del stream: {}", e);
        }
    }
}

// Controladores de solicitudes

// Función `handle_post_request` para manejar solicitudes POST
// Esta función toma una solicitud POST, extrae el cuerpo (los datos del usuario),
// y luego intenta insertarlo en la base de datos.
fn handle_post_request(request: &str) -> (String, String) {
    // Conecta con la base de datos y procesa el cuerpo de la solicitud
    match (get_user_request_body(&request), Client::connect(&obtener_url_basedatos(), NoTls)) {
        (Ok(user), Ok(mut client)) => {
            // Inserta el nuevo usuario en la tabla `users`
            client.execute(
                "INSERT INTO users (name, email) VALUES ($1, $2)", 
                &[&user.name, &user.email]
            ).unwrap();

            // Respuesta exitosa
            (OK_RESPONSE.to_string(), "User created".to_string())
        }
        // En caso de error, devuelve una respuesta de error interno
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}
// Función `handle_get_request`
// Esta función maneja una solicitud GET para obtener un usuario por su ID.
// Toma la solicitud `request` como cadena y extrae el ID del usuario.
// Realiza una consulta a la base de datos para buscar al usuario por el ID proporcionado.
// Si encuentra el usuario, lo devuelve en formato JSON con un código de respuesta HTTP 200 (OK).
// Si el usuario no se encuentra, devuelve una respuesta HTTP 404 (Not Found).
// Si hay un error en la consulta o la conexión, devuelve una respuesta HTTP 500 (Internal Server Error).
fn handle_get_request(request: &str) -> (String, String) {
    match (get_id(&request).parse::<i32>(), Client::connect(&obtener_url_basedatos(), NoTls)) {
        // Si el ID se puede parsear y la conexión a la base de datos es exitosa
        (Ok(id), Ok(mut client)) => {
            match client.query("SELECT * FROM user WHERE id = $1", &[&id]) {
                Ok(rows) => {
                    // Si se encuentra al menos una fila (el usuario existe)
                    if let Some(row) = rows.get(0) {
                        let user = User {
                            id: row.get(0),
                            name: row.get(1),
                            email: row.get(2),
                        };
                        // Devuelve el usuario en formato JSON
                        (OK_RESPONSE.to_string(), serde_json::to_string(&user).unwrap())
                    } else {
                        // Si no se encuentra el usuario
                        (NOT_FOUND.to_string(), "User not found".to_string())
                    }
                }
                // Error en la consulta SQL
                Err(_) => (INTERNAL_SERVER_ERROR.to_string(), "Error executing query".to_string()),
            }
        }
        // Error en la conexión a la base de datos o al parsear el ID
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

// Función `handle_get_all_request`
// Esta función maneja una solicitud GET para obtener todos los usuarios.
// Realiza una consulta a la base de datos para seleccionar todos los usuarios.
// Los devuelve en formato JSON con un código de respuesta HTTP 200 (OK).
// Si hay un error en la conexión o en la consulta, devuelve una respuesta HTTP 500 (Internal Server Error).
fn handle_get_all_request(request: &str) -> (String, String) {
    match Client::connect(&obtener_url_basedatos(), NoTls) {
        Ok(mut client) => {
            let mut users = Vec::new();
            // Consulta todos los usuarios y los almacena en un vector
            for row in client.query("SELECT * FROM users", &[]).unwrap() {
                users.push(User {
                    id: row.get(0),
                    name: row.get(1),
                    email: row.get(2),
                });
            }
            // Devuelve la lista de usuarios en formato JSON
            (OK_RESPONSE.to_string(), serde_json::to_string(&users).unwrap())
        }
        // Error en la conexión a la base de datos
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

// Función `handle_put_request`
// Esta función maneja una solicitud PUT para actualizar un usuario.
// Extrae el ID del usuario y el cuerpo de la solicitud para obtener los nuevos datos.
// Realiza una actualización en la base de datos con los nuevos valores del usuario.
// Devuelve una respuesta HTTP 200 (OK) si la actualización es exitosa.
// Si hay un error en la conexión, en la consulta o al parsear el ID, devuelve una respuesta HTTP 500 (Internal Server Error).
fn handle_put_request(request: &str) -> (String, String) {
    match (get_id(&request).parse::<i32>(), get_user_request_body(&request), Client::connect(&obtener_url_basedatos(), NoTls)) {
        // Si el ID, los nuevos datos y la conexión a la base de datos son válidos
        (Ok(id), Ok(user), Ok(mut client)) => {
            client.execute(
                "UPDATE user SET name=$1, email=$2 where id=$3",
                &[&user.name, &user.email, &id],
            ).unwrap();
            // Devuelve un mensaje de éxito
            (OK_RESPONSE.to_string(), "User Updated".to_string())
        }
        // Error en la conexión a la base de datos, parseo del ID o deserialización de los datos
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

// Función `handle_delete_request`
// Esta función maneja una solicitud DELETE para eliminar un usuario por su ID.
// Extrae el ID del usuario de la solicitud y realiza una eliminación en la base de datos.
// Si la eliminación es exitosa, devuelve una respuesta HTTP 200 (OK).
// Si no se encuentra el usuario, devuelve una respuesta HTTP 404 (Not Found).
// Si hay un error en la conexión o en la consulta, devuelve una respuesta HTTP 500 (Internal Server Error).
fn handle_delete_request(request: &str) -> (String, String) {
    match (get_id(&request).parse::<i32>(), Client::connect(&obtener_url_basedatos(), NoTls)) {
        // Si el ID es válido y la conexión a la base de datos es exitosa
        (Ok(id), Ok(mut client)) => {
            match client.execute("DELETE FROM users WHERE id = $1", &[&id]) {
                // Si no se eliminó ninguna fila, significa que el usuario no se encontró
                Ok(row_affected) => {
                    if row_affected == 0 {
                        return (NOT_FOUND.to_string(), "User not found".to_string());
                    }
                    // Devuelve un mensaje de éxito si el usuario fue eliminado
                    (OK_RESPONSE.to_string(), "User Deleted".to_string())
                }
                // Error al ejecutar la consulta SQL
                Err(_) => (INTERNAL_SERVER_ERROR.to_string(), "Error deleting user".to_string()),
            }
        }
        // Error en la conexión a la base de datos o al parsear el ID
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error connecting to database or parsing ID".to_string()),
    }
}

// Función `set_database`
// Esta función se encarga de configurar la base de datos PostgreSQL y crear la tabla `user` si no existe.
// Retorna un `Result` indicando si la operación fue exitosa o si ocurrió un error con la base de datos.
fn set_database() -> Result<(), PostgresError> {
    // Conexión con la base de datos
    // Se obtiene la URL de la base de datos mediante `obtener_url_basedatos` y se establece la conexión con PostgreSQL.
    let url_basedatos = obtener_url_basedatos();
    let mut client = Client::connect(&url_basedatos, NoTls)?;

    // Crear la tabla `user` si no existe
    // La tabla tiene tres columnas: `id` (llave primaria autoincremental), `name` y `email`.
    client.execute(
        "CREATE TABLE IF NOT EXISTS user (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            email VARCHAR NOT NULL
        )", &[]
    )?;
    
    Ok(()) // Retorna éxito si la tabla se creó correctamente o ya existía
}

// Función `get_id`
// Esta función extrae el ID de la solicitud HTTP.
// Toma una cadena `request` que representa la solicitud y devuelve el ID que sigue a `/users/`.
// Si no se encuentra el ID, devuelve una cadena vacía.
fn get_id(request: &str) -> &str {
    request.split("/")  // Divide la cadena por `/`
           .nth(2)      // Obtiene el tercer elemento (el ID después de `/users/`)
           .unwrap_or_default()  // Si no existe el tercer elemento, devuelve una cadena vacía
           .split_whitespace()  // Elimina posibles espacios en blanco
           .next()  // Toma el primer elemento después de eliminar los espacios
           .unwrap_or_default()  // Si no hay un elemento, devuelve una cadena vacía
}

// Función `get_user_request_body`
// Esta función deserializa el cuerpo de la solicitud HTTP en un objeto `User`.
// Toma una cadena `request` que contiene la solicitud completa y extrae la parte después del doble salto de línea (`\r\n\r\n`).
// Utiliza `serde_json` para convertir la cadena JSON en un objeto `User`.
// Retorna un `Result` que contiene el `User` deserializado o un error de deserialización de `serde_json`.
fn get_user_request_body(request: &str) -> Result<User, serde_json::Error> {
    serde_json::from_str(
        request.split("\r\n\r\n")  // Divide la solicitud en encabezado y cuerpo
               .last()  // Obtiene la última parte (el cuerpo de la solicitud)
               .unwrap_or_default()  // Si no hay cuerpo, devuelve una cadena vacía
    )
}
```
