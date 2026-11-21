//! Persistence tests: the configuration must survive a round trip through SQLite.

use mqtt_core::models::{
    MqttConnection, MqttSubscription, MqttVersion, PayloadFormat, PublishTemplate,
};
use mqtt_storage::{ConnectionRepo, Database, SettingsRepo, SubscriptionRepo, TemplateRepo};

fn connection() -> MqttConnection {
    let mut config = MqttConnection::new("conn-1", "Local", "127.0.0.1", 1883);
    config.protocol = MqttVersion::V5;
    config.username = Some("user".to_string());
    config.credential_ref = Some("conn-1".to_string());
    config.tls = true;
    config.ca_certificate = Some("/etc/ssl/ca.pem".to_string());
    config
}

#[test]
fn saves_and_loads_connections() {
    let database = Database::open_in_memory().expect("the database opens");
    let repo = ConnectionRepo::new(&database);

    repo.save(&connection()).expect("the connection is saved");

    let loaded = repo.list().expect("the connections are listed");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0], connection());
}

#[test]
fn updates_a_connection_in_place() {
    let database = Database::open_in_memory().expect("the database opens");
    let repo = ConnectionRepo::new(&database);

    repo.save(&connection()).expect("the connection is saved");

    let mut updated = connection();
    updated.port = 8883;
    repo.save(&updated).expect("the connection is updated");

    let loaded = repo.list().expect("the connections are listed");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].port, 8883);
}

#[test]
fn deletes_a_connection() {
    let database = Database::open_in_memory().expect("the database opens");
    let repo = ConnectionRepo::new(&database);

    repo.save(&connection()).expect("the connection is saved");
    repo.delete("conn-1").expect("the connection is deleted");

    assert!(repo.list().expect("the connections are listed").is_empty());
}

#[test]
fn saves_and_loads_subscriptions() {
    let database = Database::open_in_memory().expect("the database opens");
    let repo = SubscriptionRepo::new(&database);

    let mut subscription = MqttSubscription::new("sub-1", "conn-1", "device/+/status", 1);
    subscription.alias = Some("Device status".to_string());
    subscription.format = PayloadFormat::Json;

    repo.save(&subscription).expect("the subscription is saved");

    let loaded = repo
        .list_for_connection("conn-1")
        .expect("the subscriptions are listed");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].alias.as_deref(), Some("Device status"));
    assert_eq!(loaded[0].format, PayloadFormat::Json);

    repo.delete("sub-1").expect("the subscription is deleted");
    assert!(
        repo.list()
            .expect("the subscriptions are listed")
            .is_empty()
    );
}

#[test]
fn saves_and_loads_templates() {
    let database = Database::open_in_memory().expect("the database opens");
    let repo = TemplateRepo::new(&database);

    let template = PublishTemplate {
        id: "tpl-1".to_string(),
        name: "Start command".to_string(),
        topic: "device/001/cmd".to_string(),
        payload: vec![0x01, 0x02, 0xff],
        format: PayloadFormat::Hex,
        qos: 1,
        retain: false,
    };

    repo.save(&template).expect("the template is saved");

    let loaded = repo.list().expect("the templates are listed");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].payload, vec![0x01, 0x02, 0xff]);
}

#[test]
fn saves_and_loads_settings() {
    let database = Database::open_in_memory().expect("the database opens");
    let repo = SettingsRepo::new(&database);

    assert_eq!(repo.get("cache_limit").expect("the setting is read"), None);

    repo.set("cache_limit", "5000")
        .expect("the setting is saved");
    assert_eq!(
        repo.get("cache_limit").expect("the setting is read"),
        Some("5000".to_string())
    );

    repo.delete("cache_limit").expect("the setting is deleted");
    assert_eq!(repo.get("cache_limit").expect("the setting is read"), None);
}
