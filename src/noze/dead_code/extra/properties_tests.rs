use super::*;
use crate::spine::parser::parse_file;
use std::fs;

fn symbols(src: &str) -> Vec<String> {
    symbols_for(&[("m.py", src)])
}

fn symbols_for(sources: &[(&str, &str)]) -> Vec<String> {
    let tmp = tempfile::tempdir().unwrap();
    let files: Vec<_> = sources
        .iter()
        .enumerate()
        .map(|(index, (name, src))| {
            let file = tmp.path().join(name);
            fs::write(&file, src).unwrap();
            parse_file(&file, index as u32).unwrap()
        })
        .collect();
    unused_properties(&files, &HashMap::new())
        .iter()
        .map(|f| f.symbol.clone())
        .collect()
}

#[test]
fn reports_only_properties_without_class_aware_liveness() {
    let found = symbols(
        "class User:\n    name: str\n    label: str\n    dead: bool\n\n\
         def title(self):\n        return self.name\n\n\
         def render(user: User):\n    return user.label\n",
    );

    assert_eq!(found, vec!["User.dead"]);
}

#[test]
fn string_literals_keep_dynamic_property_uses_alive() {
    let found = symbols(
        "class User:\n    dynamic: str\n    stale: str\n\n\
         def read(user):\n    return getattr(user, \"dynamic\")\n",
    );

    assert_eq!(found, vec!["User.stale"]);
}

#[test]
fn returned_object_attribute_access_keeps_property_live() {
    let found = symbols(
        "class Foo:\n    a: int\n    b: int\n\n\
         def get_foo() -> Foo:\n    return Foo(a=1, b=2)\n\n\
         value = get_foo().a\n",
    );

    assert_eq!(found, vec!["Foo.b"]);
}

#[test]
fn constructed_instance_attribute_access_keeps_property_live() {
    let found = symbols(
        "class Foo:\n    a: int\n    b: int\n\n\
         instance_1 = Foo()\nvalue = instance_1.a\n",
    );

    assert_eq!(found, vec!["Foo.b"]);
}

#[test]
fn basemodel_instance_attribute_access_keeps_only_accessed_property_live() {
    let found = symbols(
        "from pydantic import BaseModel\n\n\
         class Foo(BaseModel):\n    a: int\n    b: int\n\n\
         s = Foo(2)\ntest = s.a\n",
    );

    assert_eq!(found, vec!["Foo.b"]);
}

#[test]
fn basemodel_schema_without_in_repo_usage_does_not_report_fields() {
    let found = symbols(
        "from pydantic import BaseModel\n\n\
         class Foo(BaseModel):\n    a: int\n    b: int\n",
    );

    assert_eq!(found, Vec::<String>::new());
}

#[test]
fn basemodel_field_annotations_do_not_count_as_usage_evidence() {
    let found = symbols(
        "from pydantic import BaseModel\n\n\
         class Child(BaseModel):\n    value: int\n\n\
         class Parent(BaseModel):\n    child: Child\n",
    );

    assert_eq!(found, Vec::<String>::new());
}

#[test]
fn basemodel_construction_alone_does_not_report_schema_fields() {
    let found = symbols(
        "from pydantic import BaseModel\n\n\
         class Foo(BaseModel):\n    a: int\n    b: int\n\n\
         value = Foo(a=1, b=2)\n",
    );

    assert_eq!(found, Vec::<String>::new());
}

#[test]
fn cross_file_typed_function_access_keeps_property_live() {
    let found = symbols_for(&[
        ("models.py", "class Foo:\n    a: int\n    b: int\n"),
        (
            "service.py",
            "from models import Foo\n\n\
             def use_foo(foo: Foo):\n    return foo.a\n",
        ),
        (
            "main.py",
            "from models import Foo\nfrom service import use_foo\n\n\
             instance_1 = Foo()\nresult = use_foo(instance_1)\n",
        ),
    ]);

    assert_eq!(found, vec!["Foo.b"]);
}

#[test]
fn cross_file_untyped_function_access_keeps_property_live() {
    let found = symbols_for(&[
        ("models.py", "class Foo:\n    a: int\n    b: int\n"),
        ("service.py", "def use_foo(foo):\n    return foo.a\n"),
        (
            "main.py",
            "from models import Foo\nfrom service import use_foo\n\n\
             instance_1 = Foo()\nresult = use_foo(instance_1)\n",
        ),
    ]);

    assert_eq!(found, vec!["Foo.b"]);
}

#[test]
fn untyped_attribute_access_does_not_keep_duplicate_property_names_live() {
    let found = symbols(
        "class Foo:\n    a: int\n    b: int\n\n\
         class Bar:\n    a: int\n\n\
         def read(value):\n    return value.a\n",
    );

    assert_eq!(found, vec!["Foo.a", "Foo.b", "Bar.a"]);
}

#[test]
fn constructor_with_keyword_args_still_infers_type_for_receiver() {
    let found = symbols(
        "class Foo:\n    a: int\n    b: int\n\n\
         fee = Foo(a=2, b=3)\nvalue = fee.a\n",
    );

    assert_eq!(found, vec!["Foo.b"]);
}

#[test]
fn base_class_property_access_keeps_subclass_override_live() {
    let found = symbols(
        "class Base:\n    setting: dict\n\n\
         class Child(Base):\n    setting: dict\n    stale: str\n\n\
         cmd: Base = Child()\nvalue = cmd.setting\n",
    );

    assert_eq!(found, vec!["Child.stale"]);
}

#[test]
fn typed_instance_attribute_chain_keeps_property_live() {
    let found = symbols(
        "class Key:\n    name: str\n    stale: str\n\n\
         class Other:\n    name: str\n\n\
         class Holder:\n    def __init__(self):\n        self.model: Key = Key()\n    def read(self):\n        return self.model.name\n",
    );
    assert_eq!(found, vec!["Key.stale", "Other.name"]);
}

#[test]
fn attribute_passed_as_function_argument_keeps_property_live() {
    let found = symbols(
        "class Foo:\n    a: int\n    dead: str\n\n\
         fee = Foo(a=2, dead='x')\n\
         result = check_something(fee.a)\n",
    );

    assert_eq!(found, vec!["Foo.dead"]);
}

#[test]
fn attribute_used_in_conditional_keeps_property_live() {
    let found = symbols(
        "class Foo:\n    a: int\n    dead: str\n\n\
         fee = Foo(a=2, dead='x')\n\
         if check(fee.a):\n    pass\n",
    );

    assert_eq!(found, vec!["Foo.dead"]);
}

#[test]
fn chained_attr_access_as_only_use_keeps_leaf_property_live() {
    let found = symbols(
        "class Foo:\n    a: int\n    b: str\n\n\
         class Wrapper:\n    inner: Foo\n\n\
         fee = Foo(a=2, b='x')\n\
         w = Wrapper(inner=fee)\n\
         test = w.inner.a\n",
    );

    assert_eq!(
        found,
        vec!["Foo.b"],
        "only Foo.b is dead (constructor kwarg is not usage)"
    );
}

#[test]
fn chained_attr_alongside_direct_access_both_keep_property_live() {
    let found = symbols(
        "from dataclasses import dataclass\n\n\
         @dataclass\nclass Foo:\n    a: int\n\n\
         fee = Foo(a=2)\n\
         s = fee.a\n\n\
         class Wrapper:\n    inner: Foo\n\n\
         w = Wrapper(inner=fee)\n\
         test = w.inner.a\n",
    );

    assert_eq!(found, Vec::<String>::new());
}

#[test]
fn triple_nested_chained_access_keeps_leaf_live() {
    let found = symbols(
        "class Inner:\n    value: int\n\n\
         class Middle:\n    inner: Inner\n\n\
         class Outer:\n    middle: Middle\n\n\
         i = Inner(value=1)\n\
         m = Middle(inner=i)\n\
         o = Outer(middle=m)\n\
         x = o.middle.inner.value\n",
    );

    assert_eq!(found, Vec::<String>::new());
}

#[test]
fn framework_managed_fields_and_unused_basemodel_schemas_are_skipped() {
    let found = symbols(
        "from pydantic import BaseModel, BaseSettings\n\
         from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column\n\n\
         class Settings(BaseSettings):\n    api_key: str\n\n\
         class Payload(BaseModel):\n    user_id: int\n\n\
         class Base(DeclarativeBase):\n    pass\n\n\
         class User(Base):\n    id: Mapped[int] = mapped_column(primary_key=True)\n    email: Mapped[str]\n\n\
         class Plain:\n    stale: str\n",
    );
    assert_eq!(found, vec!["Plain.stale"]);
}

#[test]
fn dataclass_and_attrs_plain_fields_are_skipped() {
    let found = symbols(
        "from dataclasses import dataclass\nimport attrs\n\n\
         @dataclass\nclass UserDto:\n    name: str\n    email: str\n\n\
         @attrs.define\nclass AttrDto:\n    token: str\n\n\
         class Plain:\n    stale: str\n",
    );

    assert_eq!(found, vec!["Plain.stale"]);
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
        vec![
            "ApiShape.public_name",
            "ApiShape.retries",
            "LegacyTable.name",
            "Plain.stale"
        ]
    );
}
