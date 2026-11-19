use super::*;

fn eps() -> HashSet<String> {
    ["route", "fixture", "task"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn classify_one(p: &str) -> DecoratorClass {
    classify(Some(&vec![p.to_string()]), &eps())
}

#[test]
fn registration_shapes() {
    assert!(matches!(
        classify_one("app.get"),
        DecoratorClass::Registration
    ));
    assert!(matches!(
        classify_one("router.websocket"),
        DecoratorClass::Registration
    ));
    assert!(matches!(
        classify_one("pytest.fixture"),
        DecoratorClass::Registration
    ));
    assert!(matches!(classify_one("task"), DecoratorClass::Registration));
}

#[test]
fn neutral_and_unknown() {
    assert!(matches!(classify_one("property"), DecoratorClass::Neutral));
    assert!(matches!(
        classify_one("functools.lru_cache"),
        DecoratorClass::Neutral
    ));
    assert!(matches!(classify_one("lru_cache"), DecoratorClass::Unknown));
    assert!(matches!(
        classify_one("my_wrapper"),
        DecoratorClass::Unknown
    ));
    assert!(matches!(classify(None, &eps()), DecoratorClass::None));
}

#[test]
fn pydantic_config_members_are_framework_managed() {
    let property = ClassProperty {
        name: "model_config".to_string(),
        ..ClassProperty::default()
    };
    let class = ClassUnit {
        bases: vec!["pydantic.BaseConfig".to_string()],
        ..ClassUnit::default()
    };

    assert!(manages_property(&property));
    assert!(manages_class_properties(&class, None));
}
