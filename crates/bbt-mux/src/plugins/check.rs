//! P-0 conformance and explicit, threshold-free measurements.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

use bbt_plugin::{
    Refusal,
    contracts::Contract,
    shape::{PluginName, SCHEMA, Shape},
};
use serde::Serialize;
use serde_json::{Value, json};

use super::{client::Client, failure, trust::FixtureAccount};

const CALL: Duration = Duration::from_secs(2);

/// A checker report describes fixture execution, never production approval.
#[derive(Debug, Serialize)]
pub struct CheckReport {
    pub ok: bool,
    pub scope: &'static str,
    pub sha256: Option<String>,
    pub checks: Vec<String>,
    pub refusal: Option<Refusal>,
    pub owned_pids: Vec<u32>,
    pub shape_runs: usize,
    pub stderr_truncated: bool,
}

impl Default for CheckReport {
    fn default() -> Self {
        Self {
            ok: false,
            scope: "isolated-conformance-fixture",
            sha256: None,
            checks: Vec::new(),
            refusal: None,
            owned_pids: Vec::new(),
            shape_runs: 0,
            stderr_truncated: false,
        }
    }
}

fn validate(schema: &Value, value: &Value, code: &str) -> Result<(), Refusal> {
    let validator =
        jsonschema::validator_for(schema).map_err(|e| failure("invalid_json_schema", e))?;
    validator.validate(value).map_err(|e| failure(code, e))
}

async fn descriptor(account: &mut FixtureAccount, report: &mut CheckReport) -> Result<(), Refusal> {
    if account.restore_shape()? {
        return Ok(());
    }
    let mut process = Client::spawn(account, true)?;
    report.owned_pids.push(process.pid);
    account.shape_runs += 1;
    let value = process.shape().await;
    let stderr = process.shutdown().await;
    report.stderr_truncated |= stderr.truncated;
    let mut value = value?;
    // The host itself must tolerate minor fields, including nested commands.
    if let Some(fields) = value.as_object_mut() {
        fields.insert("p0_future_minor_field".into(), json!(true));
    }
    account.cache_shape(parse_descriptor(value, &account.record.name)?)?;
    report.checks.push(
        "shape, versions, method names, fixed authority, hook ceilings and tool-name bounds".into(),
    );
    Ok(())
}

/// Pure descriptor checks shared by the fixture checker and production trust.
pub(super) fn parse_descriptor(value: Value, name: &PluginName) -> Result<Shape, Refusal> {
    validate(
        &serde_json::from_str(SCHEMA).expect("bundled shape schema"),
        &value,
        "invalid_shape",
    )?;
    let shape: Shape = serde_json::from_value(value).map_err(|e| failure("invalid_shape", e))?;
    shape.validate(name)?;
    for system in &shape.systems {
        let contract = Contract::named(&system.contract)?;
        for command in &system.commands {
            if contract.method_schema(&command.name, false).is_none()
                || contract.method_schema(&command.name, true).is_none()
            {
                return Err(failure(
                    "unknown_method",
                    format!("{} {}", system.contract, command.name),
                ));
            }
            jsonschema::validator_for(&command.args)
                .map_err(|e| failure("invalid_argument_schema", e))?;
            jsonschema::validator_for(&command.result)
                .map_err(|e| failure("invalid_result_schema", e))?;
        }
    }
    Ok(shape)
}

async fn tools(client: &mut Client) -> Result<BTreeMap<String, Value>, Refusal> {
    let mut collected = BTreeMap::new();
    let mut cursor = None;
    let mut cursors = std::collections::BTreeSet::new();
    loop {
        let page = client
            .request(
                "tools/list",
                cursor
                    .as_ref()
                    .map_or_else(|| json!({}), |c| json!({"cursor":c})),
                CALL,
            )
            .await?;
        let tools = page["tools"]
            .as_array()
            .ok_or_else(|| failure("invalid_tool_catalogue", "tools must be an array"))?;
        for tool in tools {
            let name = tool["name"]
                .as_str()
                .ok_or_else(|| failure("invalid_tool_catalogue", "tool has no name"))?;
            if collected.insert(name.to_owned(), tool.clone()).is_some() {
                return Err(failure("duplicate_wire_tool", name));
            }
            if collected.len() > 128 {
                return Err(failure(
                    "tool_catalogue_too_large",
                    "P-0 admits at most 128 tools",
                ));
            }
        }
        cursor = page
            .get("nextCursor")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let Some(next) = &cursor else {
            return Ok(collected);
        };
        if !cursors.insert(next.clone()) || cursors.len() > 128 {
            return Err(failure("tool_cursor_cycle", "tools/list did not terminate"));
        }
    }
}

fn typed_data_result(method: &str, value: &Value) -> Result<(), Refusal> {
    use bbt_plugin::data_source as d;
    fn decode<T: serde::de::DeserializeOwned>(value: &Value) -> Result<(), Refusal> {
        serde_json::from_value::<T>(value.clone())
            .map(|_| ())
            .map_err(|e| failure("result_type_mismatch", e))
    }
    match method {
        "resolve" => decode::<d::RecordRef>(value),
        "fetch" => decode::<d::Record>(value),
        "search" => decode::<d::Page<d::SearchHit>>(value),
        "list" => decode::<d::Page<d::RecordRef>>(value),
        "links" => decode::<d::Links>(value),
        "graph" => decode::<d::GraphPage>(value),
        "write" => decode::<d::WriteResult>(value),
        "changes_since" => decode::<d::Changes>(value),
        _ => Err(failure("unknown_method", method)),
    }
}

async fn data_call(
    client: &mut Client,
    account: &FixtureAccount,
    tools: &BTreeMap<String, Value>,
    method: &str,
    args: Value,
) -> Result<Value, Refusal> {
    let contract = Contract::DataSource;
    let wire = contract.wire_name(method)?;
    let system = account
        .record
        .shape
        .as_ref()
        .expect("validated descriptor")
        .systems
        .iter()
        .find(|s| s.contract == contract.name())
        .expect("data_source is present");
    let command = system
        .commands
        .iter()
        .find(|c| c.name == method)
        .expect("all canonical methods validated");
    let advertised = tools
        .get(&wire)
        .ok_or_else(|| failure("missing_wire_tool", &wire))?;
    validate(
        &contract
            .method_schema(method, false)
            .expect("known request"),
        &args,
        "invalid_conformance_request",
    )?;
    validate(&command.args, &args, "argument_schema_mismatch")?;
    validate(
        &advertised["inputSchema"],
        &args,
        "argument_schema_mismatch",
    )?;
    let budget = Duration::from_millis(command.timeout_ms).min(CALL);
    let original_args = args.clone();
    let result = client.call(&wire, args.clone(), budget).await?;
    let mut extended = args;
    extended["p0_future_minor_field"] = json!({"ignore":true});
    validate(&command.args, &extended, "unknown_field_rejected")?;
    validate(
        &advertised["inputSchema"],
        &extended,
        "unknown_field_rejected",
    )?;
    let extended_result = client.call(&wire, extended, budget).await.map_err(|e| {
        if e.code == "command_error" {
            failure(
                "unknown_field_rejected",
                format!("{wire} rejected an additive argument"),
            )
        } else {
            e
        }
    })?;
    for value in [&result, &extended_result] {
        validate(
            &contract.method_schema(method, true).expect("known result"),
            value,
            "result_schema_mismatch",
        )?;
        validate(&command.result, value, "result_schema_mismatch")?;
        if let Some(schema) = advertised.get("outputSchema") {
            validate(schema, value, "result_schema_mismatch")?;
        }
        let mut host_extension = value.clone();
        host_extension["p0_future_minor_field"] = json!(true);
        validate(
            &contract.method_schema(method, true).expect("known result"),
            &host_extension,
            "host_unknown_field_rejected",
        )?;
        typed_data_result(method, &host_extension)?;
        if method == "write"
            && value["lossy"] != true
            && (value["record"]["frontmatter"] != original_args["frontmatter"]
                || value["record"]["body"] != original_args["body"])
        {
            return Err(failure(
                "unreported_conversion_loss",
                "write changed Markdown/frontmatter without lossy:true",
            ));
        }
    }
    Ok(result)
}

async fn contract_call(
    client: &mut Client,
    account: &FixtureAccount,
    tools: &BTreeMap<String, Value>,
    contract: Contract,
    method: &str,
    args: Value,
) -> Result<Value, Refusal> {
    let wire = contract.wire_name(method)?;
    let advertised = tools
        .get(&wire)
        .ok_or_else(|| failure("missing_wire_tool", &wire))?;
    let system = account
        .record
        .shape
        .as_ref()
        .expect("validated descriptor")
        .systems
        .iter()
        .find(|s| s.contract == contract.name())
        .expect("contract is present");
    let command = system
        .commands
        .iter()
        .find(|c| c.name == method)
        .expect("canonical methods checked");
    validate(
        &contract
            .method_schema(method, false)
            .expect("known request"),
        &args,
        "invalid_conformance_request",
    )?;
    let signed = args
        .get("signed_record")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let mut extended = args.clone();
    let original_args = args.clone();
    extended["p0_future_minor_field"] = json!(true);
    let mut first_result = None;
    for (arguments, future) in [(args, false), (extended, true)] {
        for schema in [&command.args, &advertised["inputSchema"]] {
            validate(
                schema,
                &arguments,
                if future {
                    "unknown_field_rejected"
                } else {
                    "argument_schema_mismatch"
                },
            )?;
        }
        let result = client
            .call(
                &wire,
                arguments,
                Duration::from_millis(command.timeout_ms).min(CALL),
            )
            .await
            .map_err(|e| {
                if future && e.code == "command_error" {
                    failure("unknown_field_rejected", &wire)
                } else {
                    e
                }
            })?;
        if let Some(signed) = &signed {
            let returned = result
                .get("signed_record")
                .and_then(Value::as_str)
                .ok_or_else(|| failure("signed_record_missing", &wire))?;
            if returned.as_bytes() != signed.as_bytes() {
                return Err(failure("signed_record_changed", &wire));
            }
        }
        if contract == Contract::DecisionModel {
            let criteria = &original_args["criteria"];
            if !result["choice"]
                .as_str()
                .is_some_and(|name| criteria.get(name).is_some())
            {
                return Err(failure(
                    "decision_choice_unknown",
                    "choice must name a supplied criterion",
                ));
            }
            if result["probabilities"]
                .as_object()
                .is_some_and(|values| values.keys().any(|name| criteria.get(name).is_none()))
            {
                return Err(failure(
                    "decision_probability_unknown",
                    "probabilities must name supplied criteria",
                ));
            }
        }
        if contract == Contract::EventSink && result["id"] != original_args["envelope"]["id"] {
            return Err(failure(
                "envelope_id_changed",
                "sink deduplication key must be the envelope id",
            ));
        }
        validate(
            &contract.method_schema(method, true).expect("known result"),
            &result,
            "result_schema_mismatch",
        )?;
        validate(&command.result, &result, "result_schema_mismatch")?;
        if let Some(schema) = advertised.get("outputSchema") {
            validate(schema, &result, "result_schema_mismatch")?;
        }
        let mut extended_result = result.clone();
        extended_result["p0_future_minor_field"] = json!(true);
        validate(
            &contract.method_schema(method, true).expect("known result"),
            &extended_result,
            "host_unknown_field_rejected",
        )?;
        if !future {
            first_result = Some(result);
        }
    }
    Ok(first_result.expect("base request validated"))
}

fn signed_envelope_fixture() -> Result<Value, Refusal> {
    use crate::messaging::wire::{self, EnvelopeExt};
    use bbt_proto::{
        messaging::{Body, Envelope, IssuerKind, MessageKind},
        registry::{Class, Role},
    };
    let key = ed25519_dalek::SigningKey::from_bytes(&[73; 32]);
    let public = wire::public(&key.verifying_key().to_bytes());
    let id = crate::Ulid::from(7_u128);
    let mut envelope = Envelope {
        version: 1,
        id,
        issuer_kind: IssuerKind::LocalDev,
        issuer: public.clone(),
        account: public.clone(),
        device: public.clone(),
        seat: id,
        author: public,
        swarm_id: id,
        process_generation: 1,
        context: "fixtures/p0".into(),
        role: Role {
            name: "executor".into(),
            class: Class::Worker,
        },
        role_grant_id: "a".repeat(64),
        role_epoch: 1,
        priority_class: Class::Worker,
        sender_sequence: 1,
        thread_id: id,
        reply_to: None,
        timestamp: 1,
        priority: 0,
        kind: MessageKind::Text,
        body: Body::Text {
            text: "P-0 fixture only".into(),
        },
        attachments: vec![],
        door: None,
        issued_by: None,
        signature: String::new(),
    };
    envelope
        .sign(&key)
        .map_err(|e| failure("fixture_record_unavailable", e))?;
    serde_json::to_value(envelope).map_err(|e| failure("fixture_record_unavailable", e))
}

async fn exercise(
    client: &mut Client,
    account: &FixtureAccount,
    report: &mut CheckReport,
) -> Result<(), Refusal> {
    client.initialize(account).await?;
    let catalogue = tools(client).await?;
    let shape = account.record.shape.as_ref().expect("validated descriptor");
    for system in &shape.systems {
        let contract = Contract::named(&system.contract)?;
        for method in contract.methods() {
            let wire = contract.wire_name(method)?;
            let tool = catalogue
                .get(&wire)
                .ok_or_else(|| failure("missing_wire_tool", wire))?;
            jsonschema::validator_for(&tool["inputSchema"])
                .map_err(|e| failure("invalid_argument_schema", e))?;
            if let Some(schema) = tool.get("outputSchema") {
                jsonschema::validator_for(schema)
                    .map_err(|e| failure("invalid_result_schema", e))?;
            }
        }
    }
    client.call(&shape.health_command, json!({}), CALL).await?;
    report
        .checks
        .push("MCP negotiation, bounded catalogue and health".into());
    if shape
        .systems
        .iter()
        .any(|s| s.contract == Contract::DataSource.name())
    {
        let reference = data_call(
            client,
            account,
            &catalogue,
            "resolve",
            json!({"locator":"#7"}),
        )
        .await?;
        for (method, args) in [
            ("fetch", json!({"ref":reference})),
            ("links", json!({"ref":reference})),
            ("search", json!({"query":"plugin conformance","limit":1})),
            ("list", json!({"filter":{},"limit":1})),
            (
                "graph",
                json!({"scope":{"kind":"all","graph":"markdown"},"limit":1}),
            ),
            ("changes_since", json!({"cursor":"0","limit":1})),
            (
                "write",
                json!({"path":"p0-conformance.md","frontmatter":{"type":"note","title":"Plugin conformance"},"body":"P-0 conformance\n"}),
            ),
        ] {
            let result = data_call(client, account, &catalogue, method, args.clone()).await?;
            if method == "write"
                && result["lossy"] != true
                && (result["record"]["frontmatter"] != args["frontmatter"]
                    || result["record"]["body"] != args["body"])
            {
                return Err(failure(
                    "unreported_conversion_loss",
                    "write changed Markdown/frontmatter without lossy:true",
                ));
            }
        }
        report.checks.push("eight data_source methods; provider and host additive-field tolerance; lossless write or explicit loss".into());
    }
    if shape
        .systems
        .iter()
        .any(|s| s.contract == Contract::TaskSystem.name())
    {
        let signed = account.signed_task_fixture()?;
        for (method, args) in [
            ("list", json!({"limit":1})),
            ("status", json!({"task_path":"tasks/p0-fixture.md"})),
            ("changes_since", json!({"cursor":"0","limit":1})),
            (
                "delegate",
                json!({"task_path":"tasks/p0-fixture.md","seat_id":crate::Ulid::from(7_u128).to_string(),"etag":"fixture-etag","signed_record":signed}),
            ),
            (
                "reassign",
                json!({"task_path":"tasks/p0-fixture.md","seat_id":crate::Ulid::from(7_u128).to_string(),"etag":"fixture-etag","signed_record":signed}),
            ),
            (
                "report",
                json!({"task_path":"tasks/p0-fixture.md","summary":"fixture only","verify_result":{"command":"true","exit_code":0},"etag":"fixture-etag","signed_record":signed}),
            ),
            (
                "accept",
                json!({"task_path":"tasks/p0-fixture.md","etag":"fixture-etag","signed_record":signed}),
            ),
            (
                "reject",
                json!({"task_path":"tasks/p0-fixture.md","reason":"fixture only","etag":"fixture-etag","signed_record":signed}),
            ),
        ] {
            contract_call(
                client,
                account,
                &catalogue,
                Contract::TaskSystem,
                method,
                args,
            )
            .await?;
        }
        report.checks.push("task_system fixture methods; app-owned DSSE fixture bytes preserved on every write; no production transition or acceptance performed".into());
    }
    for system in &shape.systems {
        let contract = Contract::named(&system.contract)?;
        match contract {
            Contract::Session | Contract::Provider => {
                contract_call(client, account, &catalogue, contract, "describe", json!({})).await?;
            }
            Contract::RepoGraph => {
                contract_call(
                    client,
                    account,
                    &catalogue,
                    contract,
                    "describe",
                    json!({"root":account.record.data_dir}),
                )
                .await?;
                contract_call(
                    client,
                    account,
                    &catalogue,
                    contract,
                    "changes_since",
                    json!({"cursor":"0","limit":1}),
                )
                .await?;
            }
            Contract::DecisionModel => {
                let criteria =
                    json!({"first":"Fixture candidate one","second":"Fixture candidate two"});
                let result=contract_call(client,account,&catalogue,contract,"choose",json!({"state":{"fixture":true},"instructions":"Select one named fixture candidate","criteria":criteria})).await?;
                if !result["choice"]
                    .as_str()
                    .is_some_and(|name| criteria.get(name).is_some())
                {
                    return Err(failure(
                        "decision_choice_unknown",
                        "choice must name a supplied criterion",
                    ));
                }
                if result["probabilities"]
                    .as_object()
                    .expect("validated probabilities")
                    .keys()
                    .any(|name| criteria.get(name).is_none())
                {
                    return Err(failure(
                        "decision_probability_unknown",
                        "probabilities must name supplied criteria",
                    ));
                }
            }
            Contract::EventSink => {
                let envelope = signed_envelope_fixture()?;
                let result = contract_call(
                    client,
                    account,
                    &catalogue,
                    contract,
                    "deliver",
                    json!({"subject":"fixtures/p0","envelope":envelope}),
                )
                .await?;
                if result["id"] != envelope["id"] {
                    return Err(failure(
                        "envelope_id_changed",
                        "sink deduplication key must be the envelope id",
                    ));
                }
            }
            Contract::DataSource | Contract::TaskSystem => continue,
        }
        report.checks.push(format!(
            "{} fixture calls and additive fields",
            contract.name()
        ));
    }
    Ok(())
}

async fn prepared(account: &mut FixtureAccount) -> CheckReport {
    let mut report = CheckReport {
        sha256: Some(account.record.sha256.clone()),
        ..Default::default()
    };
    let result = async {
        descriptor(account, &mut report).await?;
        let mut process = Client::spawn(account, false)?;
        report.owned_pids.push(process.pid);
        let result = exercise(&mut process, account, &mut report).await;
        let captured = process.shutdown().await;
        report.stderr_truncated |= captured.truncated;
        // Captured plugin logs are deliberately not copied into public results.
        let _ = captured.text;
        result
    }
    .await;
    report.shape_runs = account.shape_runs;
    report.ok = result.is_ok();
    report.refusal = result.err();
    report
}

/// Run only on an IO thread; all state and child processes belong to a temp root.
pub async fn check(binary: &Path) -> CheckReport {
    match FixtureAccount::prepare(binary) {
        Ok(mut account) => prepared(&mut account).await,
        Err(error) => CheckReport {
            refusal: Some(error),
            ..Default::default()
        },
    }
}

/// Raw samples and definitions, with no latency pass/fail threshold.
#[derive(Debug, Serialize)]
pub struct Measurement {
    pub conformance: CheckReport,
    pub cold_us: Vec<u64>,
    pub warm_us: Vec<u64>,
    pub cold_p50_us: u64,
    pub cold_p99_us: u64,
    pub warm_p50_us: u64,
    pub warm_p99_us: u64,
    pub load_before: [f64; 3],
    pub load_after: [f64; 3],
    pub definition: &'static str,
    pub os: &'static str,
    pub arch: &'static str,
}

fn load() -> [f64; 3] {
    let mut values = [0.0; 3];
    // SAFETY: the array has exactly the three entries requested from libc.
    if unsafe { libc::getloadavg(values.as_mut_ptr(), 3) } != 3 {
        return [-1.0; 3];
    }
    values
}
fn percentile(samples: &[u64], percent: usize) -> u64 {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    sorted[(sorted.len() * percent).div_ceil(100) - 1]
}
fn micros(start: Instant) -> u64 {
    start.elapsed().as_micros().try_into().unwrap_or(u64::MAX)
}

async fn measured_call(client: &mut Client) -> Result<Value, Refusal> {
    client
        .call("data_source_resolve", json!({"locator":"#7"}), CALL)
        .await
}

/// Measure at least 100 cold and warm calls, reusing the one cached descriptor.
pub async fn measure(binary: &Path, samples: usize) -> Result<Measurement, Refusal> {
    if !(100..=10_000).contains(&samples) {
        return Err(failure(
            "invalid_sample_count",
            "samples must be in 100..=10000",
        ));
    }
    let mut account = FixtureAccount::prepare(binary)?;
    let mut report = prepared(&mut account).await;
    if let Some(error) = report.refusal.clone() {
        return Err(error);
    }
    if !account
        .record
        .shape
        .as_ref()
        .expect("cached descriptor")
        .systems
        .iter()
        .any(|s| s.contract == "data_source")
    {
        return Err(failure(
            "measurement_requires_data_source",
            "the cold/warm reference operation is data_source_resolve",
        ));
    }
    let before = load();
    let result_schema = Contract::DataSource
        .method_schema("resolve", true)
        .expect("known result");
    let validator =
        jsonschema::validator_for(&result_schema).map_err(|e| failure("invalid_json_schema", e))?;
    let mut cold = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        let mut process = Client::spawn(&account, false)?;
        report.owned_pids.push(process.pid);
        let result = async {
            process.initialize(&account).await?;
            measured_call(&mut process).await
        }
        .await;
        let elapsed = micros(start);
        let captured = process.shutdown().await;
        report.stderr_truncated |= captured.truncated;
        validator
            .validate(&result?)
            .map_err(|e| failure("result_schema_mismatch", e))?;
        cold.push(elapsed);
    }
    let mut process = Client::spawn(&account, false)?;
    report.owned_pids.push(process.pid);
    let mut warm = Vec::with_capacity(samples);
    let result = async {
        process.initialize(&account).await?;
        for _ in 0..samples {
            let start = Instant::now();
            let result = measured_call(&mut process).await?;
            warm.push(micros(start));
            validator
                .validate(&result)
                .map_err(|e| failure("result_schema_mismatch", e))?;
        }
        Ok::<_, Refusal>(())
    }
    .await;
    let captured = process.shutdown().await;
    report.stderr_truncated |= captured.truncated;
    result?;
    Ok(Measurement {
        cold_p50_us: percentile(&cold, 50),
        cold_p99_us: percentile(&cold, 99),
        warm_p50_us: percentile(&warm, 50),
        warm_p99_us: percentile(&warm, 99),
        cold_us: cold,
        warm_us: warm,
        load_before: before,
        load_after: load(),
        conformance: report,
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        definition: "nearest-rank p50/p99; cold = fresh spawn + MCP initialize/initialized + resolve, including pinned-copy verification; warm = resolve on one initialized child; cached --shape, result-schema validation and cleanup excluded; microseconds; no latency threshold",
    })
}

/// Early app subcommand: no GUI, supervisor, PTY, identity custody or real store.
pub fn run_cli(args: &[String]) -> i32 {
    if args.is_empty() || args.iter().any(|s| s == "--help" || s == "-h") {
        println!(
            "bboxy450-term plugin check <binary>\nbboxy450-term plugin measure <binary> [--samples N]\nRuns in an isolated fixture account; never writes production trust."
        );
        return 0;
    }
    let result = (|| {
        let [mode, binary, rest @ ..] = args else {
            return Err(failure("plugin_usage", "expected check|measure <binary>"));
        };
        let samples = match rest {
            [] => 100,
            [flag, count] if flag == "--samples" && mode == "measure" => count
                .parse()
                .map_err(|_| failure("invalid_sample_count", "expected an integer"))?,
            _ => return Err(failure("plugin_usage", "unexpected plugin arguments")),
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| failure("runtime_unavailable", e))?;
        match mode.as_str() {
            "check" => {
                let report = runtime.block_on(check(Path::new(binary)));
                let ok = report.ok;
                println!(
                    "{}",
                    serde_json::to_string(&report).expect("report serializes")
                );
                Ok(if ok { 0 } else { 1 })
            }
            "measure" => {
                let measurement = runtime.block_on(measure(Path::new(binary), samples))?;
                println!(
                    "{}",
                    serde_json::to_string(&measurement).expect("measurement serializes")
                );
                Ok(0)
            }
            _ => Err(failure("plugin_usage", "expected check|measure <binary>")),
        }
    })();
    match result {
        Ok(code) => code,
        Err(error) => {
            println!("{}", json!({"ok":false,"refusal":error}));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn expand(root: &Value, value: &Value) -> Value {
        if let Some(reference) = value.get("$ref").and_then(Value::as_str) {
            return expand(
                root,
                root.pointer(reference.strip_prefix('#').unwrap()).unwrap(),
            );
        }
        match value {
            Value::Array(items) => Value::Array(items.iter().map(|v| expand(root, v)).collect()),
            Value::Object(fields) => Value::Object(
                fields
                    .iter()
                    .map(|(k, v)| (k.clone(), expand(root, v)))
                    .collect(),
            ),
            _ => value.clone(),
        }
    }

    // keeps: spec plugins
    #[test]
    fn task_contract_properties_derive_from_the_current_native_tool_catalogue() {
        let actor = crate::mcp::Actor::Operator {
            device_id: crate::Ulid::from(1_u128),
            door: crate::mcp::Door::Gui,
            issued_by: None,
        };
        let document: Value = serde_json::from_str(Contract::TaskSystem.schema()).unwrap();
        let tools = crate::mcp::work_items::catalogue(&actor);
        let mut seen = Vec::new();
        for tool in tools {
            let Some(method) = tool["name"].as_str().unwrap().strip_prefix("task_") else {
                continue;
            };
            let schema = Contract::TaskSystem.method_schema(method, false).unwrap();
            let resolved = expand(
                &schema,
                &schema["$ref"].as_str().map(|r| json!({"$ref":r})).unwrap(),
            );
            for (name, value) in tool["inputSchema"]["properties"].as_object().unwrap() {
                if name == "op_id" {
                    continue;
                }
                assert_eq!(
                    expand(&document, &resolved["properties"][name]),
                    *value,
                    "{method}.{name} drifted from W2.5d"
                );
            }
            seen.push(method.to_owned());
        }
        seen.sort();
        assert_eq!(
            seen,
            [
                "accept", "delegate", "list", "reassign", "reject", "report", "status"
            ]
        );
    }

    #[tokio::test]
    async fn cached_descriptor_survives_a_host_reload_and_a_new_digest_has_no_cache() {
        let files = tempfile::tempdir().unwrap();
        let source = files.path().join("bboxy450-term-data_source-reference");
        let bytes =
            include_str!("../../tests/fixtures/plugins/bboxy450-term-data_source-reference");
        fs::write(&source, bytes).unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).unwrap();
        let mut account = FixtureAccount::prepare(&source).unwrap();
        let mut report = CheckReport::default();
        descriptor(&mut account, &mut report).await.unwrap();
        assert_eq!(account.shape_runs, 1);
        account.record.shape = None;
        descriptor(&mut account, &mut report).await.unwrap();
        assert_eq!(
            account.shape_runs, 1,
            "cached shape came from the account, not another process"
        );
        assert_eq!(report.owned_pids.len(), 1);
        fs::write(
            &source,
            bytes.replacen("FAULT = None", "FAULT = 'major'", 1),
        )
        .unwrap();
        let mut changed = FixtureAccount::prepare(&source).unwrap();
        assert_ne!(changed.record.sha256, account.record.sha256);
        assert!(
            !changed.restore_shape().unwrap(),
            "new bytes never inherit the previous descriptor approval"
        );
        let mut report = CheckReport::default();
        assert_eq!(
            descriptor(&mut changed, &mut report)
                .await
                .unwrap_err()
                .code,
            "major_mismatch"
        );
    }
}
