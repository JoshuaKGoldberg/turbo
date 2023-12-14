use std::collections::BTreeMap;

use biome_console::{markup, ColorMode, ConsoleExt, EnvConsole};
use biome_deserialize::{
    json::deserialize_from_json_str, Deserializable, DeserializableValue,
    DeserializationDiagnostic, DeserializationVisitor, Deserialized, Text, VisitableType,
};
use biome_diagnostics::{DiagnosticExt, PrintDiagnostic};
use biome_json_parser::JsonParserOptions;
use biome_json_syntax::TextRange;
use miette::Diagnostic;
use thiserror::Error;

use crate::{
    cli::OutputLogsMode,
    config::ConfigurationOptions,
    run::task_id::TaskName,
    turbo_json::{RawPipeline, RawTaskDefinition, RawTurboJSON, SpacesJson},
};

#[derive(Debug, Error, Diagnostic)]
enum Error {
    #[error("failed to parse turbo.json")]
    Parse {
        diagnostics: Vec<biome_diagnostics::Error>,
    },
}

fn print_diagnostics(diagnostics: &[biome_diagnostics::Error], color: bool) {
    let color_mode = if color {
        ColorMode::Enabled
    } else {
        ColorMode::Disabled
    };
    let mut console = EnvConsole::new(color_mode);
    for diagnostic in diagnostics {
        console.error(markup!({ PrintDiagnostic::simple(diagnostic) }));
    }
}

impl Deserializable for OutputLogsMode {
    fn deserialize(
        value: &impl DeserializableValue,
        name: &str,
        diagnostics: &mut Vec<DeserializationDiagnostic>,
    ) -> Option<Self> {
        match String::deserialize(value, name, diagnostics)?.as_str() {
            "full" => Some(OutputLogsMode::Full),
            "none" => Some(OutputLogsMode::None),
            "hash-only" => Some(OutputLogsMode::HashOnly),
            "new-only" => Some(OutputLogsMode::NewOnly),
            "errors-only" => Some(OutputLogsMode::ErrorsOnly),
            unknown_variant => {
                const ALLOWED_VARIANTS: &[&str] =
                    &["full", "none", "hash-only", "new-only", "errors-only"];
                diagnostics.push(DeserializationDiagnostic::new_unknown_value(
                    unknown_variant,
                    value.range(),
                    ALLOWED_VARIANTS,
                ));
                None
            }
        }
    }
}

impl Deserializable for TaskName<'static> {
    fn deserialize(
        value: &impl DeserializableValue,
        name: &str,
        diagnostics: &mut Vec<DeserializationDiagnostic>,
    ) -> Option<Self> {
        let task_id = String::deserialize(value, name, diagnostics)?;

        Some(Self::from(task_id))
    }
}

impl Deserializable for RawTaskDefinition {
    fn deserialize(
        value: &impl DeserializableValue,
        name: &str,
        diagnostics: &mut Vec<DeserializationDiagnostic>,
    ) -> Option<Self> {
        value.deserialize(RawTaskDefinitionVisitor, name, diagnostics)
    }
}

struct RawTaskDefinitionVisitor;

impl DeserializationVisitor for RawTaskDefinitionVisitor {
    type Output = RawTaskDefinition;

    const EXPECTED_TYPE: VisitableType = VisitableType::MAP;

    fn visit_map(
        self,
        // Iterator of key-value pairs.
        members: impl Iterator<Item = Option<(impl DeserializableValue, impl DeserializableValue)>>,
        // range of the map in the source text.
        range: TextRange,
        _name: &str,
        diagnostics: &mut Vec<DeserializationDiagnostic>,
    ) -> Option<Self::Output> {
        let mut result = RawTaskDefinition::default();
        for (key, value) in members.flatten() {
            let Some(key_text) = Text::deserialize(&key, "", diagnostics) else {
                continue;
            };
            match key_text.text() {
                "cache" => {
                    if let Some(cache) = bool::deserialize(&value, &key_text, diagnostics) {
                        result.cache = Some(cache);
                    }
                }
                "dependsOn" => {
                    if let Some(depends_on) = Vec::deserialize(&value, &key_text, diagnostics) {
                        result.depends_on = Some(depends_on);
                    }
                }
                "dotEnv" => {
                    if let Some(dot_env) = Vec::deserialize(&value, &key_text, diagnostics) {
                        result.dot_env = Some(dot_env);
                    }
                }
                "env" => {
                    if let Some(env) = Vec::deserialize(&value, &key_text, diagnostics) {
                        result.env = Some(env);
                    }
                }
                "inputs" => {
                    if let Some(inputs) = Vec::deserialize(&value, &key_text, diagnostics) {
                        result.inputs = Some(inputs);
                    }
                }
                "passThroughEnv" => {
                    if let Some(pass_through_env) = Vec::deserialize(&value, &key_text, diagnostics)
                    {
                        result.pass_through_env = Some(pass_through_env);
                    }
                }
                "persistent" => {
                    if let Some(persistent) = bool::deserialize(&value, &key_text, diagnostics) {
                        result.persistent = Some(persistent);
                    }
                }
                "outputs" => {
                    if let Some(outputs) = Vec::deserialize(&value, &key_text, diagnostics) {
                        result.outputs = Some(outputs);
                    }
                }
                "output_mode" => {
                    if let Some(output_mode) =
                        OutputLogsMode::deserialize(&value, &key_text, diagnostics)
                    {
                        result.output_mode = Some(output_mode);
                    }
                }
                _ => {
                    const ALLOWED_KEYS: &[&str] = &[
                        "cache",
                        "dependsOn",
                        "dotEnv",
                        "env",
                        "inputs",
                        "passThroughEnv",
                        "persistent",
                        "outputs",
                        "output_mode",
                    ];
                    diagnostics.push(DeserializationDiagnostic::new_unknown_key(
                        key_text.text(),
                        key.range(),
                        ALLOWED_KEYS,
                    ));
                }
            }
        }

        Some(result)
    }
}

impl Deserializable for SpacesJson {
    fn deserialize(
        value: &impl DeserializableValue,
        name: &str,
        diagnostics: &mut Vec<DeserializationDiagnostic>,
    ) -> Option<Self> {
        value.deserialize(SpacesJsonVisitor, name, diagnostics)
    }
}

struct SpacesJsonVisitor;

impl DeserializationVisitor for SpacesJsonVisitor {
    type Output = SpacesJson;

    const EXPECTED_TYPE: VisitableType = VisitableType::MAP;

    fn visit_map(
        self,
        members: impl Iterator<Item = Option<(impl DeserializableValue, impl DeserializableValue)>>,
        range: TextRange,
        _name: &str,
        diagnostics: &mut Vec<DeserializationDiagnostic>,
    ) -> Option<Self::Output> {
        let mut result = SpacesJson::default();
        for (key, value) in members.flatten() {
            let Some(key_text) = Text::deserialize(&key, "", diagnostics) else {
                continue;
            };
            match key_text.text() {
                "id" => {
                    if let Some(id) = String::deserialize(&value, &key_text, diagnostics) {
                        result.id = Some(id);
                    }
                }
                // We don't care about other keys currently
                _ => {}
            }
        }
        Some(result)
    }
}

impl Deserializable for ConfigurationOptions {
    fn deserialize(
        value: &impl DeserializableValue,
        name: &str,
        diagnostics: &mut Vec<DeserializationDiagnostic>,
    ) -> Option<Self> {
        value.deserialize(ConfigurationOptionsVisitor, name, diagnostics)
    }
}

struct ConfigurationOptionsVisitor;

impl DeserializationVisitor for ConfigurationOptionsVisitor {
    type Output = ConfigurationOptions;

    const EXPECTED_TYPE: VisitableType = VisitableType::MAP;

    fn visit_map(
        self,
        // Iterator of key-value pairs.
        members: impl Iterator<Item = Option<(impl DeserializableValue, impl DeserializableValue)>>,
        // range of the map in the source text.
        range: TextRange,
        _name: &str,
        diagnostics: &mut Vec<DeserializationDiagnostic>,
    ) -> Option<Self::Output> {
        let mut result = ConfigurationOptions::default();
        for (key, value) in members.flatten() {
            // Try to deserialize the key as a string.
            // We use `Text` to avoid an heap-allocation.
            let Some(key_text) = Text::deserialize(&key, "", diagnostics) else {
                // If this failed, then pass to the next key-value pair.
                continue;
            };
            match key_text.text() {
                "apiUrl" | "apiurl" | "ApiUrl" | "APIURL" => {
                    if let Some(api_url) = String::deserialize(&value, &key_text, diagnostics) {
                        result.api_url = Some(api_url);
                    }
                }
                "loginUrl" | "loginurl" | "LoginUrl" | "LOGINURL" => {
                    if let Some(login_url) = String::deserialize(&value, &key_text, diagnostics) {
                        result.login_url = Some(login_url);
                    }
                }
                "teamSlug" | "teamslug" | "TeamSlug" | "TEAMSLUG" => {
                    if let Some(team_slug) = String::deserialize(&value, &key_text, diagnostics) {
                        result.team_slug = Some(team_slug);
                    }
                }
                "teamId" | "teamid" | "TeamId" | "TEAMID" => {
                    if let Some(team_id) = String::deserialize(&value, &key_text, diagnostics) {
                        result.team_id = Some(team_id);
                    }
                }
                "token" => {
                    if let Some(token) = String::deserialize(&value, &key_text, diagnostics) {
                        result.token = Some(token);
                    }
                }
                "signature" => {
                    if let Some(signature) = bool::deserialize(&value, &key_text, diagnostics) {
                        result.signature = Some(signature);
                    }
                }
                "preflight" => {
                    if let Some(preflight) = bool::deserialize(&value, &key_text, diagnostics) {
                        result.preflight = Some(preflight);
                    }
                }
                "timeout" => {
                    if let Some(timeout) = u64::deserialize(&value, &key_text, diagnostics) {
                        result.timeout = Some(timeout);
                    }
                }
                "enabled" => {
                    if let Some(enabled) = bool::deserialize(&value, &key_text, diagnostics) {
                        result.enabled = Some(enabled);
                    }
                }
                unknown_key => {
                    const ALLOWED_KEYS: &[&str] = &[
                        "apiUrl",
                        "loginUrl",
                        "teamSlug",
                        "teamId",
                        "token",
                        "signature",
                        "preflight",
                        "timeout",
                        "enabled",
                    ];
                    diagnostics.push(DeserializationDiagnostic::new_unknown_key(
                        unknown_key,
                        key.range(),
                        ALLOWED_KEYS,
                    ))
                }
            }
        }

        Some(result)
    }
}

impl Deserializable for RawTurboJSON {
    fn deserialize(
        value: &impl DeserializableValue,
        name: &str,
        diagnostics: &mut Vec<DeserializationDiagnostic>,
    ) -> Option<Self> {
        value.deserialize(RawTurboJsonVisitor, name, diagnostics)
    }
}

struct RawTurboJsonVisitor;

impl DeserializationVisitor for RawTurboJsonVisitor {
    type Output = RawTurboJSON;

    const EXPECTED_TYPE: VisitableType = VisitableType::MAP;

    fn visit_map(
        self,
        // Iterator of key-value pairs.
        members: impl Iterator<Item = Option<(impl DeserializableValue, impl DeserializableValue)>>,
        // range of the map in the source text.
        range: TextRange,
        _name: &str,
        diagnostics: &mut Vec<DeserializationDiagnostic>,
    ) -> Option<Self::Output> {
        let mut result = RawTurboJSON::default();
        for (key, value) in members.flatten() {
            // Try to deserialize the key as a string.
            // We use `Text` to avoid an heap-allocation.
            let Some(key_text) = Text::deserialize(&key, "", diagnostics) else {
                // If this failed, then pass to the next key-value pair.
                continue;
            };
            match key_text.text() {
                "schema" => {
                    if let Some(name) = String::deserialize(&value, &key_text, diagnostics) {
                        result.schema = Some(name);
                    }
                }
                "extends" => {
                    if let Some(extends) = Vec::deserialize(&value, &key_text, diagnostics) {
                        result.extends = Some(extends);
                    }
                }
                "globalDependencies" => {
                    if let Some(global_dependencies) =
                        Vec::deserialize(&value, &key_text, diagnostics)
                    {
                        result.global_dependencies = Some(global_dependencies);
                    }
                }
                "globalEnv" => {
                    if let Some(global_env) = Vec::deserialize(&value, &key_text, diagnostics) {
                        result.global_env = Some(global_env);
                    }
                }
                "globalPassThroughEnv" => {
                    if let Some(global_pass_through_env) =
                        Vec::deserialize(&value, &key_text, diagnostics)
                    {
                        result.global_pass_through_env = Some(global_pass_through_env);
                    }
                }
                "globalDotEnv" => {
                    if let Some(global_dot_env) = Vec::deserialize(&value, &key_text, diagnostics) {
                        result.global_dot_env = Some(global_dot_env);
                    }
                }
                "experimentalSpaces" => {
                    if let Some(spaces) = SpacesJson::deserialize(&value, &key_text, diagnostics) {
                        result.experimental_spaces = Some(spaces);
                    }
                }
                "pipeline" => {
                    if let Some(pipeline) = BTreeMap::deserialize(&value, &key_text, diagnostics) {
                        result.pipeline = Some(RawPipeline(pipeline));
                    }
                }
                "remoteCache" => {
                    if let Some(remote_cache) =
                        ConfigurationOptions::deserialize(&value, &key_text, diagnostics)
                    {
                        result.remote_cache = Some(remote_cache);
                    }
                }
                unknown_key => {
                    const ALLOWED_KEYS: &[&str] = &[
                        "schema",
                        "extends",
                        "globalDependencies",
                        "globalEnv",
                        "globalPassThroughEnv",
                        "globalDotEnv",
                        "experimentalSpaces",
                        "pipeline",
                        "remoteCache",
                    ];
                    diagnostics.push(DeserializationDiagnostic::new_unknown_key(
                        unknown_key,
                        key.range(),
                        ALLOWED_KEYS,
                    ));
                }
            }
        }
        Some(result)
    }
}

fn parse_turbo_json(text: &str, file_path: &str) -> Result<RawTurboJSON, Error> {
    let result = deserialize_from_json_str::<RawTurboJSON>(
        text,
        JsonParserOptions::default().with_allow_comments(),
    );

    if !result.diagnostics().is_empty() {
        let diagnostics = result
            .into_diagnostics()
            .into_iter()
            .map(|d| d.with_file_source_code(text).with_file_path(file_path))
            .collect();

        return Err(Error::Parse { diagnostics });
    }

    Ok(result
        .into_deserialized()
        .expect("should have turbo.json value if no errors"))
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use miette::Report;

    use super::Error;
    use crate::turbo_json::parser::{parse_turbo_json, print_diagnostics};

    #[test]
    fn test_parse_turbo() {
        let text = r#"{
         "extends": ["base"],
         "experimentalSpaces": { "id": "foobar" },
         "pipeline": {
           "build": { "dependsOn": ["lint"] }
         }
        }"#;
        let result = parse_turbo_json(text, "turbo.json").unwrap();
        println!("{:?}", result);
    }

    #[test]
    fn test_root_not_object() {
        let text = r#"10"#;
        let error = parse_turbo_json(text, "turbo.json").unwrap_err();
        match error {
            Error::Parse { diagnostics } => {
                print_diagnostics(&diagnostics, true);
            }
            err => println!("{:?}", Report::new(err)),
        }
    }
}
