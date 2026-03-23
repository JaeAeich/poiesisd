use std::collections::HashMap;

use askama::Template;
use axum::Form;
use axum::extract::State;
use axum::response::{Html, Redirect};
use sqlx::SqlitePool;

use crate::database;
use crate::dto::{TesExecutor, TesInput, TesOutput, TesTask};

#[derive(Template)]
#[template(path = "task_form.html")]
struct TaskFormTemplate;

pub async fn task_form() -> Html<String> {
    let tmpl = TaskFormTemplate;
    Html(
        tmpl.render()
            .unwrap_or_else(|e| format!("Template error: {e}")),
    )
}

pub async fn create_task(
    State(pool): State<SqlitePool>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Redirect, Html<String>> {
    let task = match parse_task_form(&form) {
        Ok(t) => t,
        Err(e) => return Err(Html(format!("Invalid form data: {e}"))),
    };

    match database::insert_task(&pool, &task).await {
        Ok(id) => Ok(Redirect::to(&format!("/tasks/{id}"))),
        Err(e) => Err(Html(format!("Failed to create task: {e}"))),
    }
}

fn parse_task_form(form: &HashMap<String, String>) -> Result<TesTask, String> {
    let name = form.get("name").filter(|s| !s.is_empty()).cloned();
    let description = form.get("description").filter(|s| !s.is_empty()).cloned();

    // Parse executors
    let mut executors: Vec<TesExecutor> = Vec::new();
    for i in 0..100 {
        let image_key = format!("executor_{i}_image");
        let command_key = format!("executor_{i}_command");

        let image = match form.get(&image_key).filter(|s| !s.is_empty()) {
            Some(img) => img.clone(),
            None => break,
        };

        let command_str = form
            .get(&command_key)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("executor {i}: command is required"))?;

        let command: Vec<String> = command_str.split_whitespace().map(String::from).collect();

        let workdir = form
            .get(&format!("executor_{i}_workdir"))
            .filter(|s| !s.is_empty())
            .cloned();
        let stdin = form
            .get(&format!("executor_{i}_stdin"))
            .filter(|s| !s.is_empty())
            .cloned();
        let stdout = form
            .get(&format!("executor_{i}_stdout"))
            .filter(|s| !s.is_empty())
            .cloned();
        let stderr = form
            .get(&format!("executor_{i}_stderr"))
            .filter(|s| !s.is_empty())
            .cloned();

        let env = form
            .get(&format!("executor_{i}_env"))
            .filter(|s| !s.is_empty())
            .map(|env_str| {
                env_str
                    .lines()
                    .filter_map(|line| {
                        let line = line.trim();
                        if line.is_empty() {
                            return None;
                        }
                        let (k, v) = line.split_once('=')?;
                        Some((k.trim().to_string(), v.trim().to_string()))
                    })
                    .collect::<HashMap<String, String>>()
            })
            .filter(|m| !m.is_empty());

        let ignore_error = form
            .get(&format!("executor_{i}_ignore_error"))
            .map(|v| v == "true");

        executors.push(TesExecutor {
            image,
            command,
            workdir,
            stdin,
            stdout,
            stderr,
            env,
            ignore_error,
        });
    }

    if executors.is_empty() {
        return Err("at least one executor is required".to_string());
    }

    // Parse inputs
    let mut inputs: Vec<TesInput> = Vec::new();
    for i in 0..100 {
        let url = form
            .get(&format!("input_{i}_url"))
            .filter(|s| !s.is_empty());
        let path = form
            .get(&format!("input_{i}_path"))
            .filter(|s| !s.is_empty());

        match (url, path) {
            (Some(u), Some(p)) => {
                inputs.push(TesInput {
                    name: form
                        .get(&format!("input_{i}_name"))
                        .filter(|s| !s.is_empty())
                        .cloned(),
                    description: None,
                    url: Some(u.clone()),
                    path: p.clone(),
                    r#type: None,
                    content: None,
                    streamable: None,
                });
            }
            (None, None) => break,
            _ => continue,
        }
    }

    // Parse outputs
    let mut outputs: Vec<TesOutput> = Vec::new();
    for i in 0..100 {
        let url = form
            .get(&format!("output_{i}_url"))
            .filter(|s| !s.is_empty());
        let path = form
            .get(&format!("output_{i}_path"))
            .filter(|s| !s.is_empty());

        match (url, path) {
            (Some(u), Some(p)) => {
                outputs.push(TesOutput {
                    name: form
                        .get(&format!("output_{i}_name"))
                        .filter(|s| !s.is_empty())
                        .cloned(),
                    description: None,
                    url: u.clone(),
                    path: p.clone(),
                    path_prefix: None,
                    r#type: None,
                });
            }
            (None, None) => break,
            _ => continue,
        }
    }

    // Parse volumes
    let volumes: Option<Vec<String>> = form
        .get("volumes")
        .filter(|s| !s.is_empty())
        .map(|v| {
            v.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
        .filter(|v: &Vec<String>| !v.is_empty());

    // Parse tags
    let tags: Option<HashMap<String, String>> = form
        .get("tags")
        .filter(|s| !s.is_empty())
        .map(|t| {
            t.lines()
                .filter_map(|line| {
                    let line = line.trim();
                    if line.is_empty() {
                        return None;
                    }
                    let (k, v) = line.split_once('=')?;
                    Some((k.trim().to_string(), v.trim().to_string()))
                })
                .collect()
        })
        .filter(|m: &HashMap<String, String>| !m.is_empty());

    Ok(TesTask {
        name,
        description,
        executors,
        inputs: if inputs.is_empty() {
            None
        } else {
            Some(inputs)
        },
        outputs: if outputs.is_empty() {
            None
        } else {
            Some(outputs)
        },
        volumes,
        tags,
        ..Default::default()
    })
}
