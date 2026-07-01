use super::*;
use crate::spine::parser::parse_file;
use std::fs;

fn symbols(src: &str) -> Vec<String> {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("m.py");
    fs::write(&file, src).unwrap();
    let files = vec![parse_file(&file, 0).unwrap()];
    unused_properties(&files, &HashMap::new())
        .iter()
        .map(|f| f.symbol.clone())
        .collect()
}

#[test]
fn reports_only_properties_without_class_aware_liveness() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fs::write(
        dir.join("m.py"),
        "class User:\n    name: str\n    label: str\n    dead: bool\n\n    def title(self):\n        return self.name\n\n\ndef render(user: User):\n    return user.label\n",
    )
    .unwrap();
    let files = vec![parse_file(&dir.join("m.py"), 0).unwrap()];
    let findings = unused_properties(&files, &HashMap::new());
    let symbols: Vec<_> = findings.iter().map(|f| f.symbol.as_str()).collect();

    assert_eq!(symbols, vec!["User.dead"]);
    assert_eq!(findings[0].kind, SymbolKind::Property);
}

#[test]
fn string_literals_keep_dynamic_property_uses_alive() {
    let found = symbols(
        "class User:\n    dynamic: str\n    stale: str\n\n\ndef read(user):\n    return getattr(user, \"dynamic\")\n",
    );

    assert_eq!(found, vec!["User.stale"]);
}

#[test]
fn returned_object_attribute_access_keeps_property_live() {
    let found = symbols(
        "class Foo:\n    a: int\n    b: int\n\n\ndef get_foo() -> Foo:\n    return Foo(a=1, b=2)\n\n\nvalue = get_foo().a\n",
    );

    assert_eq!(found, vec!["Foo.b"]);
}

#[test]
fn constructed_instance_attribute_access_keeps_property_live() {
    let found = symbols(
        "class Foo:\n    a: int\n    b: int\n\n\ninstance_1 = Foo()\nvalue = instance_1.a\n",
    );

    assert_eq!(found, vec!["Foo.b"]);
}

#[test]
fn framework_managed_fields_are_skipped_but_basemodel_plain_fields_remain_candidates() {
    let found = symbols(
        "from pydantic import BaseModel, BaseSettings\nfrom sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column\n\n\
         class Settings(BaseSettings):\n    api_key: str\n\n\
         class Payload(BaseModel):\n    user_id: int\n\n\
         class Base(DeclarativeBase):\n    pass\n\n\
         class User(Base):\n    id: Mapped[int] = mapped_column(primary_key=True)\n    email: Mapped[str]\n\n\
         class Plain:\n    stale: str\n",
    );

    assert_eq!(found, vec!["Payload.user_id", "Plain.stale"]);
}

#[test]
fn dataclass_and_attrs_plain_fields_remain_candidates() {
    let found = symbols(
        "from dataclasses import dataclass\nimport attrs\n\n\
         @dataclass\nclass UserDto:\n    name: str\n    email: str\n\n\
         @attrs.define\nclass AttrDto:\n    token: str\n\n\
         class Plain:\n    stale: str\n",
    );

    assert_eq!(
        found,
        vec![
            "UserDto.name",
            "UserDto.email",
            "AttrDto.token",
            "Plain.stale"
        ]
    );
}

#[test]
fn orm_and_pydantic_field_constructors_suppress_only_those_fields() {
    let found = symbols(
        "from pydantic import Field\nfrom sqlalchemy import Column, Integer\n\n\
         class ApiShape:\n    public_name: str = Field(alias='publicName')\n    retries: int\n\n\
         class LegacyTable:\n    id = Column(Integer)\n    name: str\n\n\
         class Plain:\n    stale: str\n",
    );

    assert_eq!(
        found,
        vec!["ApiShape.retries", "LegacyTable.name", "Plain.stale"]
    );
}
