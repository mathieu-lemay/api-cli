use std::collections::HashMap;
use std::env;
use std::time::Instant;

use api_cli::error::Result;
use api_cli::{ApiClientRequest, CollectionModel, RequestModel};
use log::debug;

use super::printer::pretty_print;
use super::utils::{
    get_collection_file_path,
    get_environment_file_path,
    get_request_file_path,
    read_file,
};
use super::RunArgs;
use crate::commands::printer::json_print;
use crate::commands::OutputFormat;

pub async fn execute_request(args: RunArgs) -> Result<()> {
    let collection_path = get_collection_file_path(&args.collection);
    let collection: CollectionModel = read_file(collection_path.as_path())?;
    debug!("Collection: {:#?}", collection);

    let request_path = get_request_file_path(&args.collection, &args.request);
    let req: RequestModel = read_file(request_path.as_path())?;
    debug!("Request: {:#?}", req);

    let mut req = ApiClientRequest::new(collection, req);

    let global_variables: HashMap<String, String> = env::vars()
        .filter(|(k, _)| k.starts_with("API_CLI_VAR_"))
        .map(|(k, v)| (k.strip_prefix("API_CLI_VAR_").unwrap().to_string(), v))
        .collect();

    req = req.with_global_variables(global_variables);

    if let Some(e) = &args.environment {
        let environment_path = get_environment_file_path(&args.collection, e);
        let env = read_file(environment_path.as_path())?;
        debug!("Environment: {:#?}", env);

        req = req.with_environment(env);
    };

    let request_start = Instant::now();
    let res = req.execute().await.expect("error performing request");
    let request_duration = request_start.elapsed();

    match args.output_format {
        OutputFormat::Json => json_print(res, request_duration).await,
        OutputFormat::Pretty => pretty_print(&args, res, request_duration).await,
    }
}
