use std::{convert::Infallible, net::SocketAddr, sync::{Arc, Mutex}};

use hyper::{service::{make_service_fn, service_fn}, Body, Method, Request, Response, Server};
use serde::{Deserialize, Serialize};


#[tokio::main]
async fn main() {
    let person_list = Arc::new(Mutex::new(
        PersonList {
            list: vec![
                Person {
                    id: 0,
                    name: "Jim".to_string()
                },
                Person {
                    id: 1,
                    name: String::from("Joe")
                }
            ]
        }
    ));
    let addr = SocketAddr::from(([127,0,0,1], 3000));
    let make_svc = make_service_fn(move |_| {
        let person_list = person_list.clone();
        async move {
            Ok::<_ , Infallible>(service_fn(move |req| handle_request(req, person_list.clone())))
        }
    });
    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        println!("error: {:?}", e)
    }
}

async fn handle_request(req: Request<Body>, person_list: Arc<Mutex<PersonList>>) -> Result<Response<Body>, Infallible> {
    let response = match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => Response::new(Body::from("Hello World!")),
        (&Method::GET, "/all") => {
            let person_list =person_list.lock().unwrap();
            let list = serde_json::to_string(&(*person_list)).unwrap();
            Response::new(Body::from(list))
        },
        (&Method::GET, _) if req.uri().path().starts_with("/person/") => {
            let path_segments = req.uri().path().split("/").collect::<Vec<&str>>();
            let id = (*(path_segments.get(2).unwrap())).parse::<usize>().unwrap();
            let person_list = person_list.lock().unwrap();
            let person_op = (*person_list).list.iter().find(|person| person.id == id);
            match person_op {
                Some(person) => {
                    let person_json = serde_json::to_string(person).unwrap();
                    Response::new(Body::from(person_json))
                },
                None => Response::new(Body::from(format!("Person id : {} Not Found", id)))
            }
        },
        (&Method::POST, "/add") => {
            let body = hyper::body::to_bytes(req.into_body()).await.unwrap();
            let person_body: RequestBody = serde_json::from_slice(&body).unwrap();
            let mut person_list = person_list.lock().unwrap();
            let person = Person {
                id: if (*person_list).list.len() == 0 {0} else {(*person_list).list.last().unwrap().id + 1},
                name: person_body.name
            };
            (*person_list).list.push(person);
            Response::new(Body::from("New Person added!"))
        },
        (&Method::DELETE, _) if req.uri().path().starts_with("/delete/") => {
            let path_segments = req.uri().path().split("/").collect::<Vec<&str>>();
            let id = (*(path_segments.get(2).unwrap())).parse::<usize>().unwrap();

            let mut person_list = person_list.lock().unwrap();
            let person_op = (*person_list).list.iter().find(|person| person.id == id);
            match person_op {
                Some(_person) => {
                    let new_person_list = (*person_list).list.clone().into_iter().filter(|person| person.id != id).collect::<Vec<Person>>();
                    (*person_list).list = new_person_list;
                    Response::new(Body::from(format!("Person id : {} removed", id)))
                },
                None => Response::new(Body::from(format!("Person id : {} Not Found", id)))
            }
        },
        (&Method::PUT, _) if req.uri().path().starts_with("/update/") => {
            let path_segments = req.uri().path().split("/").collect::<Vec<&str>>();
            let id = (*path_segments.get(2).unwrap()).parse::<usize>().unwrap();

            let body = hyper::body::to_bytes(req.into_body()).await.unwrap();
            let person_body: RequestBody = serde_json::from_slice(&body).unwrap();

            let mut person_list = (*person_list).lock().unwrap();
            let person_op = (*person_list).list.iter().find(|person| person.id == id);

            match person_op {
                None => Response::new(Body::from(format!("Person id : {} not found", id))),
                Some(_person) => {
                    let new_person_list = (*person_list).list.iter().map(|person| {
                        if person.id == id {
                            Person {
                                id: person.id,
                                name: person_body.name.clone()
                            }
                        } else {
                            person.clone()
                        }
                    }).collect::<Vec<Person>>();
                    (*person_list).list = new_person_list;
                    Response::new(Body::from(format!("Person id : {} updated", id)))
                }
            }

        },
        _ => Response::new(Body::from("404 | Not Found!"))
    };
    Ok(response)
}

#[derive(Deserialize, Serialize, Clone)]
struct Person {
    id: usize,
    name: String
}
#[derive(Deserialize, Serialize)]
struct PersonList {
    list: Vec<Person>
}

#[derive(Deserialize, Serialize)]
struct RequestBody {
    name: String,
}