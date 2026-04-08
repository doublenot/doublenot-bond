use std::fs;

use serde_yaml::Value;

fn read_release_workflow() -> Value {
    let workflow =
        fs::read_to_string(".github/workflows/release.yml").expect("read release workflow");
    serde_yaml::from_str(&workflow).expect("parse release workflow")
}

fn workflow_jobs() -> serde_yaml::Mapping {
    read_release_workflow()["jobs"]
        .as_mapping()
        .expect("jobs mapping")
        .clone()
}

fn job_steps(job: &Value) -> Vec<Value> {
    job["steps"].as_sequence().expect("job steps").to_vec()
}

#[test]
fn release_workflow_uses_single_aggregate_release_job() {
    let jobs = workflow_jobs();
    let publish_job = jobs
        .get(Value::from("publish"))
        .expect("publish job should exist");

    let publish_needs = publish_job["needs"]
        .as_sequence()
        .expect("publish needs sequence");
    let publish_needs: Vec<&str> = publish_needs
        .iter()
        .map(|value| value.as_str().expect("need name"))
        .collect();
    assert_eq!(publish_needs, vec!["build", "source", "checksums"]);

    let release_job_count = jobs
        .values()
        .filter(|job| {
            job_steps(job).iter().any(|step| {
                step["uses"]
                    .as_str()
                    .is_some_and(|uses| uses == "softprops/action-gh-release@v2")
            })
        })
        .count();
    assert_eq!(
        release_job_count, 1,
        "only publish should create the release"
    );

    let publish_steps = job_steps(publish_job);
    assert!(publish_steps.iter().any(|step| {
        step["name"]
            .as_str()
            .is_some_and(|name| name == "Publish GitHub release")
    }));
}

#[test]
fn release_workflow_gates_crates_publish_and_uploads_expected_assets() {
    let jobs = workflow_jobs();
    let publish_job = jobs
        .get(Value::from("publish"))
        .expect("publish job should exist");
    let publish_steps = job_steps(publish_job);

    assert!(publish_steps.iter().any(|step| {
        step["name"]
            .as_str()
            .is_some_and(|name| name == "Validate crates.io publication readiness")
    }));
    assert!(publish_steps.iter().any(|step| {
        step["name"]
            .as_str()
            .is_some_and(|name| name == "Verify crates.io credentials")
    }));
    assert!(publish_steps.iter().any(|step| {
        step["name"]
            .as_str()
            .is_some_and(|name| name == "Publish crate to crates.io")
    }));

    let release_step = publish_steps
        .iter()
        .find(|step| {
            step["name"]
                .as_str()
                .is_some_and(|name| name == "Publish GitHub release")
        })
        .expect("publish release step");
    let files = release_step["with"]["files"]
        .as_str()
        .expect("release files glob");
    assert!(files.contains("release-artifacts/*"));

    let script =
        fs::read_to_string("scripts/release-dry-run.sh").expect("read release dry-run script");
    assert!(script.contains("cargo publish --dry-run --locked"));
}
