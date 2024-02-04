mod errors;

use std::collections::BTreeMap;

use axum::{
    extract::Path,
    response::{Html, IntoResponse},
    routing::{delete, get, post, put},
    Form, Router,
};
use errors::ApplicationError;
use leptos::*;
use serde::{Deserialize, Serialize};
use tower_http::trace::TraceLayer;
use tower_sessions::{MemoryStore, Session, SessionManagerLayer};
use tracing_subscriber::prelude::*;

const TODOS_KEY: &str = "todos";
const INDEX_KEY: &str = "index";

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tower_http=debug,axum::rejection=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);

    let app = Router::new()
        .route("/", get(root))
        .route("/todos", get(get_todos))
        .route("/todos", post(create_todo))
        .route("/todos/:id", put(put_todo))
        .route("/todos/:id", delete(delete_todo))
        .layer(TraceLayer::new_for_http())
        .layer(session_layer);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Todo {
    content: String,
    done: bool,
}

async fn root(session: Session) -> impl IntoResponse {
    if let None = session.get::<usize>(INDEX_KEY).await.unwrap() {
        let mut map = BTreeMap::new();
        map.insert(
            0,
            Todo {
                content: "A faire".into(),
                done: false,
            },
        );
        session.insert(TODOS_KEY, map).await.unwrap();
        session.insert(INDEX_KEY, 0).await.unwrap();
    }
    let todos: BTreeMap<usize, Todo> = session.get(TODOS_KEY).await.unwrap().unwrap();
    Html(
        leptos::ssr::render_to_string(|| {
            view! {
                <head>
                    <script src="https://unpkg.com/htmx.org@1.9.10"></script>
                    <script src="https://cdn.tailwindcss.com"></script>
                </head>
                <body class="w-1/2 m-auto">
                    <h1 class="text-3xl">TodoMVC</h1>
                    <select name="sort" hx-trigger="change" hx-get="/todos" hx-target="#todos">
                        <option select="selected" value="all">All</option>
                        <option value="done">Done</option>
                        <option value="not done">Not done</option>
                    </select>
                    <div class="flex flex-col" id="todos">
                        {todos.into_iter().map(|(id, todo)| view! {
                            <Todo id todo/>
                        }).collect_view()}
                    </div>
                    <hr class="w-full"/>
                    <form hx-post="/todos" hx-target="#todos" hx-swap="beforeend">
                        <input type="text" name="content"/>
                        <button
                            class="bg-teal-200 rounded-md p-2"
                            type="submit"
                        >
                            Add new
                        </button>
                    </form>
                </body>
            }
        })
        .into_owned(),
    )
}

#[derive(Deserialize)]
struct GetTodosForm {
    sort: String,
}

async fn get_todos(session: Session, Form(form): Form<GetTodosForm>) -> impl IntoResponse {
    let todos: BTreeMap<usize, Todo> = session.get(TODOS_KEY).await.unwrap().unwrap();
    let filter: Box<dyn Fn(&(usize, Todo)) -> bool> = if form.sort == "all" {
        Box::new(|_| true)
    } else if form.sort == "done" {
        Box::new(|(_, todo)| todo.done)
    } else {
        Box::new(|(_, todo)| !todo.done)
    };

    Html(
        leptos::ssr::render_to_string(move || {
            todos
                .into_iter()
                .filter(|el| filter(el))
                .map(|(id, todo)| view! {<Todo id todo />})
                .collect_view()
        })
        .into_owned(),
    )
}

async fn put_todo(
    Path(id): Path<usize>,
    session: Session,
) -> Result<impl IntoResponse, ApplicationError> {
    let mut todos: BTreeMap<usize, Todo> = session.get(TODOS_KEY).await.unwrap().unwrap();

    let todo = todos.get_mut(&id).ok_or(ApplicationError::NotFound)?;
    todo.done = !todo.done;
    let todo = todo.clone();

    session.insert(TODOS_KEY, todos).await.unwrap();

    Ok(Html(
        leptos::ssr::render_to_string(move || {
            view! {
                <Todo id=id todo=todo/>
            }
        })
        .into_owned(),
    ))
}

#[derive(Deserialize)]
struct CreateTodoForm {
    content: String,
}

async fn create_todo(session: Session, Form(form): Form<CreateTodoForm>) -> impl IntoResponse {
    let todo = Todo {
        content: form.content,
        done: false,
    };
    let mut todos: BTreeMap<usize, Todo> = session.get(TODOS_KEY).await.unwrap().unwrap();
    let i: usize = session.get(INDEX_KEY).await.unwrap().unwrap();
    session.insert(INDEX_KEY, i + 1).await.unwrap();
    todos.insert(i + 1, todo.clone());
    session.insert(TODOS_KEY, todos).await.unwrap();
    Html(leptos::ssr::render_to_string(move || view! {<Todo id=i+1 todo=todo/>}).into_owned())
}

async fn delete_todo(
    session: Session,
    Path(id): Path<usize>,
) -> Result<impl IntoResponse, ApplicationError> {
    let mut todos: BTreeMap<usize, Todo> = session.get(TODOS_KEY).await.unwrap().unwrap();
    todos.remove(&id).ok_or(ApplicationError::NotFound)?;
    session.insert(TODOS_KEY, todos).await.unwrap();
    Ok(())
}

#[component]
fn Todo(id: usize, todo: Todo) -> impl IntoView {
    view! {
        <div id={format!("todo-{id}")} class="w-full">
            <hr class="w-full"/>
            <div class="flex flex-row justify-between w-full text-xl">
                <p>{todo.content}</p>
                <p>{if todo.done { "done" } else { "not done" }}</p>
                <button
                    hx-put={format!("todos/{id}")}
                    hx-target={format!("#todo-{id}")}
                    hx-swap="outerHTML"
                >
                    Mark as done
                </button>
                <button
                    hx-delete={format!("todos/{id}")}
                    hx-target={format!("#todo-{id}")}
                    hx-swap="delete"
                >
                    Delete
                </button>
            </div>
        </div>
    }
}
