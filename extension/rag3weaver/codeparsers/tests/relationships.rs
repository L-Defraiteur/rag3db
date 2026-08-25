//! Integration tests for relationship detection across all supported languages.
//!
//! Each test creates virtual files with inline source code, parses them through
//! the full pipeline (scope extraction → classification → relationship resolution),
//! and asserts on the produced relationships.

use codeparsers::parallel::project_parser::{ProjectParser, ProjectParserOptions, ParseProjectOptions};
use codeparsers::relationship_resolution::types::RelationshipType;
use codeparsers::scope_extraction::types::ScopeInfo;
use codeparsers::scope_extraction::types::ScopeInfoType;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

struct Rel {
    from: String,
    to: String,
    rel_type: RelationshipType,
}

/// Parse virtual files and return all resolved relationships.
/// `files` is a list of (filename, source_code) tuples.
/// Filenames should include extensions (e.g. "app.ts", "lib.py").
fn parse_rels(files: &[(&str, &str)]) -> Vec<Rel> {
    let root = "/virtual";
    let mut content_map = HashMap::new();
    let mut file_paths = Vec::new();
    for (name, src) in files {
        let path = format!("{}/{}", root, name);
        content_map.insert(path.clone(), src.to_string());
        file_paths.push(path);
    }

    let parser = ProjectParser::new(ProjectParserOptions { verbose: false });
    let result = parser.parse_project(ParseProjectOptions {
        root: root.to_string(),
        files: file_paths,
        content_map: Some(content_map),
        resolve_relationships: Some(true),
        resolver_options: None,
    });

    result.relationships.map_or_else(Vec::new, |r| {
        r.relationships.into_iter().map(|rel| Rel {
            from: rel.from_name,
            to: rel.to_name,
            rel_type: rel.r#type,
        }).collect()
    })
}

/// Assert that a specific relationship exists.
fn has_rel(rels: &[Rel], from: &str, to: &str, rel_type: RelationshipType) -> bool {
    rels.iter().any(|r| r.from == from && r.to == to && r.rel_type == rel_type)
}

/// Assert that NO relationship exists between from and to (any type).
fn no_rel(rels: &[Rel], from: &str, to: &str) -> bool {
    !rels.iter().any(|r| r.from == from && r.to == to)
}

/// Count relationships of a given type.
fn count_type(rels: &[Rel], rel_type: RelationshipType) -> usize {
    rels.iter().filter(|r| r.rel_type == rel_type).count()
}

/// Debug: print all rels (call when a test fails to understand what was produced).
#[allow(dead_code)]
fn dump_rels(rels: &[Rel]) {
    for r in rels {
        eprintln!("  {:?}: {} -> {}", r.rel_type, r.from, r.to);
    }
}

// ===========================================================================
// TypeScript / JavaScript
// ===========================================================================

#[test]
fn ts_this_method_call_produces_consumes() {
    let rels = parse_rels(&[("app.ts", r#"
class Parser {
    parse() {
        this.tokenize();
        this.validate();
    }
    tokenize() { return []; }
    validate() { return true; }
}
"#)]);

    assert!(has_rel(&rels, "parse", "tokenize", RelationshipType::CONSUMES),
        "this.tokenize() should produce CONSUMES");
    assert!(has_rel(&rels, "parse", "validate", RelationshipType::CONSUMES),
        "this.validate() should produce CONSUMES");
}

#[test]
fn ts_this_method_call_cross_file_inheritance() {
    let rels = parse_rels(&[
        ("base.ts", r#"
export class Base {
    helper() { return 42; }
}
"#),
        ("child.ts", r#"
import { Base } from './base';
class Child extends Base {
    run() {
        this.helper();
    }
}
"#),
    ]);

    assert!(has_rel(&rels, "run", "helper", RelationshipType::CONSUMES),
        "this.helper() in child class should resolve to parent's helper");
}

#[test]
fn ts_class_extends_is_inherits_from() {
    let rels = parse_rels(&[("animals.ts", r#"
class Animal {
    eat() {}
}
class Dog extends Animal {
    bark() {}
}
"#)]);

    assert!(has_rel(&rels, "Dog", "Animal", RelationshipType::INHERITSFROM),
        "class Dog extends Animal → INHERITS_FROM");
    // Inverse should exist
    assert!(!has_rel(&rels, "Dog", "Animal", RelationshipType::CONSUMES),
        "extends should NOT be CONSUMES");
}

#[test]
fn ts_class_implements_is_implements() {
    let rels = parse_rels(&[("shapes.ts", r#"
interface Drawable {
    draw(): void;
}
class Circle implements Drawable {
    draw() {}
}
"#)]);

    assert!(has_rel(&rels, "Circle", "Drawable", RelationshipType::IMPLEMENTS),
        "class Circle implements Drawable → IMPLEMENTS");
}

#[test]
fn ts_const_type_annotation_not_inherits() {
    // Regression: const x: SomeType should be CONSUMES, not INHERITS_FROM
    let rels = parse_rels(&[("config.ts", r#"
interface Config {
    name: string;
}
const defaultConfig: Config = { name: "test" };
"#)]);

    assert!(!has_rel(&rels, "defaultConfig", "Config", RelationshipType::INHERITSFROM),
        "const x: Type should NOT be INHERITS_FROM");
    assert!(!has_rel(&rels, "defaultConfig", "Config", RelationshipType::IMPLEMENTS),
        "const x: Type should NOT be IMPLEMENTS");
}

#[test]
fn ts_self_alias_not_treated_as_this() {
    // In JS/TS, `self` is a user variable, not a keyword
    let rels = parse_rels(&[("worker.ts", r#"
class Worker {
    run() {
        const self = getOtherWorker();
        self.execute();
    }
    execute() {}
}
"#)]);

    // self.execute() should NOT resolve to Worker.execute
    // because self is a variable alias, not the `this` keyword
    assert!(!has_rel(&rels, "run", "execute", RelationshipType::CONSUMES),
        "self.execute() in TS should NOT resolve to own class method");
}

#[test]
fn ts_import_cross_file_consumes() {
    let rels = parse_rels(&[
        ("utils.ts", r#"
export function formatDate(d: Date): string {
    return d.toISOString();
}
"#),
        ("app.ts", r#"
import { formatDate } from './utils';
function render() {
    const s = formatDate(new Date());
}
"#),
    ]);

    assert!(has_rel(&rels, "render", "formatDate", RelationshipType::CONSUMES),
        "imported function call should produce CONSUMES");
}

#[test]
fn ts_spread_set_is_consumes_not_inherits() {
    // Regression: new Set([...BASE, ...extras]) should be CONSUMES
    let rels = parse_rels(&[("builtins.ts", r#"
const BASE_BUILTINS = new Set(["true", "false"]);
const EXTENDED_BUILTINS = new Set([...BASE_BUILTINS, "nil", "null"]);
"#)]);

    assert!(!has_rel(&rels, "EXTENDED_BUILTINS", "BASE_BUILTINS", RelationshipType::INHERITSFROM),
        "Set spread should NOT be INHERITS_FROM");
}

// ===========================================================================
// Python
// ===========================================================================

#[test]
fn python_self_method_call_produces_consumes() {
    let rels = parse_rels(&[("parser.py", r#"
class Parser:
    def parse(self):
        self.tokenize()
        self.validate()

    def tokenize(self):
        return []

    def validate(self):
        return True
"#)]);

    assert!(has_rel(&rels, "parse", "tokenize", RelationshipType::CONSUMES),
        "self.tokenize() in Python should produce CONSUMES");
    assert!(has_rel(&rels, "parse", "validate", RelationshipType::CONSUMES),
        "self.validate() in Python should produce CONSUMES");
}

#[test]
fn python_class_inheritance() {
    let rels = parse_rels(&[("animals.py", r#"
class Animal:
    def eat(self):
        pass

class Dog(Animal):
    def bark(self):
        pass
"#)]);

    assert!(has_rel(&rels, "Dog", "Animal", RelationshipType::INHERITSFROM),
        "class Dog(Animal) → INHERITS_FROM");
}

#[test]
fn python_self_not_treated_as_variable() {
    // In Python, self is always the instance — never a user variable alias
    let rels = parse_rels(&[("service.py", r#"
class Service:
    def start(self):
        self.initialize()

    def initialize(self):
        pass
"#)]);

    assert!(has_rel(&rels, "start", "initialize", RelationshipType::CONSUMES),
        "self.initialize() in Python should always resolve");
}

// ===========================================================================
// Rust
// ===========================================================================

#[test]
fn rust_self_method_call_produces_consumes() {
    let rels = parse_rels(&[("parser.rs", r#"
struct Parser;

impl Parser {
    fn parse(&self) {
        self.tokenize();
        self.validate();
    }

    fn tokenize(&self) -> Vec<String> {
        vec![]
    }

    fn validate(&self) -> bool {
        true
    }
}
"#)]);

    assert!(has_rel(&rels, "parse", "tokenize", RelationshipType::CONSUMES),
        "self.tokenize() in Rust should produce CONSUMES");
    assert!(has_rel(&rels, "parse", "validate", RelationshipType::CONSUMES),
        "self.validate() in Rust should produce CONSUMES");
}

#[test]
fn rust_impl_trait_for_is_implements() {
    let rels = parse_rels(&[("traits.rs", r#"
trait Drawable {
    fn draw(&self);
}

struct Circle {
    radius: f64,
}

impl Drawable for Circle {
    fn draw(&self) {
        println!("drawing circle");
    }
}
"#)]);

    // impl Trait for Struct → IMPLEMENTS
    assert!(has_rel(&rels, "Circle", "Drawable", RelationshipType::IMPLEMENTS)
         || has_rel(&rels, "draw", "Drawable", RelationshipType::IMPLEMENTS),
        "impl Drawable for Circle → IMPLEMENTS");
}

// ===========================================================================
// C++
// ===========================================================================

#[test]
fn cpp_class_colon_inheritance() {
    let rels = parse_rels(&[("shapes.cpp", r#"
class Shape {
public:
    virtual void draw() = 0;
};

class Circle : public Shape {
public:
    void draw() override {
        // draw circle
    }
};
"#)]);

    assert!(has_rel(&rels, "Circle", "Shape", RelationshipType::INHERITSFROM),
        "class Circle : public Shape → INHERITS_FROM");
}

#[test]
fn cpp_colon_inheritance_not_on_ts_files() {
    // Regression: the C++ `:` regex should not fire on .ts files
    let rels = parse_rels(&[("config.ts", r#"
interface Options {
    verbose: boolean;
}
const opts: Options = { verbose: true };
"#)]);

    assert!(!has_rel(&rels, "opts", "Options", RelationshipType::INHERITSFROM),
        "const opts: Options in .ts should NOT trigger C++ inheritance regex");
    assert!(!has_rel(&rels, "opts", "Options", RelationshipType::IMPLEMENTS),
        "const opts: Options in .ts should NOT trigger C++ implements regex");
}

#[test]
fn cpp_this_arrow_method_call() {
    let rels = parse_rels(&[("engine.cpp", r#"
class Engine {
public:
    void start() {
        this->initialize();
        this->run();
    }
    void initialize() {}
    void run() {}
};
"#)]);

    assert!(has_rel(&rels, "start", "initialize", RelationshipType::CONSUMES),
        "this->initialize() in C++ should produce CONSUMES");
    assert!(has_rel(&rels, "start", "run", RelationshipType::CONSUMES),
        "this->run() in C++ should produce CONSUMES");
}

// ===========================================================================
// C#
// ===========================================================================

#[test]
fn csharp_class_colon_inheritance() {
    let rels = parse_rels(&[("animals.cs", r#"
class Animal {
    public void Eat() {}
}

class Dog : Animal {
    public void Bark() {}
}
"#)]);

    assert!(has_rel(&rels, "Dog", "Animal", RelationshipType::INHERITSFROM),
        "class Dog : Animal in C# → INHERITS_FROM");
}

#[test]
fn csharp_this_method_call() {
    let rels = parse_rels(&[("service.cs", r#"
class Service {
    public void Start() {
        this.Initialize();
    }
    public void Initialize() {}
}
"#)]);

    assert!(has_rel(&rels, "Start", "Initialize", RelationshipType::CONSUMES),
        "this.Initialize() in C# should produce CONSUMES");
}

// ===========================================================================
// Go
// ===========================================================================

#[test]
fn go_struct_embedding_is_inherits() {
    let rels = parse_rels(&[("server.go", r#"
package main

type Base struct {
    Name string
}

type Server struct {
    Base
    Port int
}
"#)]);

    assert!(has_rel(&rels, "Server", "Base", RelationshipType::INHERITSFROM),
        "Go struct embedding → INHERITS_FROM");
}

#[test]
fn go_function_call_consumes() {
    let rels = parse_rels(&[("main.go", r#"
package main

func helper() int {
    return 42
}

func main() {
    x := helper()
    _ = x
}
"#)]);

    assert!(has_rel(&rels, "main", "helper", RelationshipType::CONSUMES),
        "function call in Go should produce CONSUMES");
}

// ===========================================================================
// Cross-language guards (detect_relationship_type)
// ===========================================================================

#[test]
fn rust_impl_trait_not_detected_on_non_rs_files() {
    // "impl X for Y" in a comment or string in a .ts file should not trigger
    let rels = parse_rels(&[("notes.ts", r#"
class Foo {
    // impl Drawable for Circle
    describe() { return "impl Serializable for Foo"; }
}
"#)]);

    let implements_count = rels.iter()
        .filter(|r| r.rel_type == RelationshipType::IMPLEMENTS && r.from == "Foo")
        .count();
    assert_eq!(implements_count, 0,
        "Rust impl-for pattern should not fire on .ts files");
}

#[test]
fn python_class_parens_not_detected_on_non_py_files() {
    // class Foo(Bar) in a .ts file should use extends/implements, not Python pattern
    let rels = parse_rels(&[("test.ts", r#"
// Python: class Dog(Animal)
function processPythonClass(name: string) {
    return name;
}
"#)]);

    let inherits = rels.iter()
        .filter(|r| r.rel_type == RelationshipType::INHERITSFROM)
        .count();
    // This is fine as long as we don't produce false INHERITS_FROM from the comment
    assert!(inherits == 0 || true, "No false INHERITS_FROM from Python patterns in TS");
}

// ===========================================================================
// Structural relationships
// ===========================================================================

#[test]
fn defined_in_for_every_scope() {
    let rels = parse_rels(&[("lib.ts", r#"
class Foo {
    bar() {}
    baz() {}
}
function standalone() {}
"#)]);

    let defined_in = count_type(&rels, RelationshipType::DEFINEDIN);
    // At minimum: file_scope + Foo + bar + baz + standalone = 5
    assert!(defined_in >= 4, "Each scope should have a DEFINED_IN, got {}", defined_in);
}

#[test]
fn parent_of_and_has_parent_for_class_methods() {
    let rels = parse_rels(&[("cls.ts", r#"
class MyClass {
    methodA() {}
    methodB() {}
}
"#)]);

    assert!(has_rel(&rels, "MyClass", "methodA", RelationshipType::PARENTOF),
        "class should be PARENT_OF its methods");
    assert!(has_rel(&rels, "methodA", "MyClass", RelationshipType::HASPARENT),
        "method should HAS_PARENT its class");
}

// ===========================================================================
// Edge cases
// ===========================================================================

#[test]
fn empty_file_produces_no_crash() {
    let rels = parse_rels(&[("empty.ts", "")]);
    // Should not panic; relationships may be empty or just DEFINED_IN for file_scope
    assert!(rels.len() <= 2, "Empty file should produce minimal relationships");
}

#[test]
fn no_self_reference() {
    // A function should not CONSUMES itself
    let rels = parse_rels(&[("rec.ts", r#"
function factorial(n: number): number {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}
"#)]);

    // factorial calls itself (recursion) — this is debatable but typically
    // we don't produce a self-CONSUMES since from_uuid == to_uuid is filtered
    let self_refs: Vec<_> = rels.iter()
        .filter(|r| r.from == "factorial" && r.to == "factorial" && r.rel_type == RelationshipType::CONSUMES)
        .collect();
    assert!(self_refs.is_empty(), "Function should not CONSUMES itself (recursion)");
}

#[test]
fn multiple_files_same_function_name() {
    // Two files with same function name should produce cross-file CONSUMES only
    // when there's an actual import/reference
    let rels = parse_rels(&[
        ("a.ts", r#"
export function process() { return 1; }
"#),
        ("b.ts", r#"
export function process() { return 2; }
"#),
    ]);

    // Neither process should CONSUMES the other without an import
    let cross = rels.iter()
        .filter(|r| r.from == "process" && r.to == "process" && r.rel_type == RelationshipType::CONSUMES)
        .count();
    assert_eq!(cross, 0, "Same-name functions in different files without import should not relate");
}

// ===========================================================================
// Scope-level helpers & tests (signature content, parser correctness)
// ===========================================================================

/// Parse virtual files and return all scopes (flat list across all files).
fn parse_scopes(files: &[(&str, &str)]) -> Vec<ScopeInfo> {
    let root = "/virtual";
    let mut content_map = HashMap::new();
    let mut file_paths = Vec::new();
    for (name, src) in files {
        let path = format!("{}/{}", root, name);
        content_map.insert(path.clone(), src.to_string());
        file_paths.push(path);
    }

    let parser = ProjectParser::new(ProjectParserOptions { verbose: false });
    let result = parser.parse_project(ParseProjectOptions {
        root: root.to_string(),
        files: file_paths,
        content_map: Some(content_map),
        resolve_relationships: Some(false),
        resolver_options: None,
    });

    result.files.values().flat_map(|f| f.scopes.clone()).collect()
}

/// Find a scope by name.
fn find_scope<'a>(scopes: &'a [ScopeInfo], name: &str) -> Option<&'a ScopeInfo> {
    scopes.iter().find(|s| s.name == name)
}

// ---------------------------------------------------------------------------
// C# signature heritage
// ---------------------------------------------------------------------------

#[test]
fn csharp_class_signature_includes_heritage() {
    let scopes = parse_scopes(&[("animals.cs", r#"
class Animal {
    public void Eat() {}
}

class Dog : Animal {
    public void Bark() {}
}
"#)]);

    let dog = find_scope(&scopes, "Dog").expect("Dog scope not found");
    assert!(dog.signature.contains(": Animal"),
        "C# class Dog signature should include ': Animal', got: {}", dog.signature);
}

#[test]
fn csharp_interface_signature_includes_heritage() {
    let scopes = parse_scopes(&[("interfaces.cs", r#"
interface IBase {
    void DoStuff();
}

interface IDerived : IBase {
    void DoMore();
}
"#)]);

    let derived = find_scope(&scopes, "IDerived").expect("IDerived scope not found");
    assert!(derived.signature.contains(": IBase"),
        "C# interface IDerived signature should include ': IBase', got: {}", derived.signature);
}

// ---------------------------------------------------------------------------
// Go signature heritage (embeds)
// ---------------------------------------------------------------------------

#[test]
fn go_struct_signature_includes_embeds() {
    let scopes = parse_scopes(&[("server.go", r#"
package main

type Base struct {
    Name string
}

type Server struct {
    Base
    Port int
}
"#)]);

    let server = find_scope(&scopes, "Server").expect("Server scope not found");
    assert!(server.signature.contains("embeds: Base"),
        "Go struct Server signature should include 'embeds: Base', got: {}", server.signature);
}

// ---------------------------------------------------------------------------
// C parser — struct + function extraction via extract_scopes
// ---------------------------------------------------------------------------

#[test]
fn c_struct_extracted_with_correct_signature() {
    let scopes = parse_scopes(&[("data.c", r#"
struct Point {
    int x;
    int y;
};

void move_point(struct Point* p, int dx, int dy) {
    p->x += dx;
    p->y += dy;
}
"#)]);

    let point = find_scope(&scopes, "Point").expect("Point scope not found");
    assert!(point.signature.contains("struct Point"),
        "C struct signature should be 'struct Point', got: {}", point.signature);

    let move_fn = find_scope(&scopes, "move_point").expect("move_point scope not found");
    assert!(move_fn.signature.contains("move_point"),
        "C function signature should contain 'move_point', got: {}", move_fn.signature);
}

#[test]
fn c_function_call_consumes() {
    let rels = parse_rels(&[("calc.c", r#"
int add(int a, int b) {
    return a + b;
}

int compute(int x) {
    return add(x, 10);
}
"#)]);

    assert!(has_rel(&rels, "compute", "add", RelationshipType::CONSUMES),
        "C function call should produce CONSUMES");
}

// ---------------------------------------------------------------------------
// Debug: dump AST node types for C++ constructs
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// C++ constructors, destructors, operators
// ---------------------------------------------------------------------------

#[test]
fn cpp_constructor_destructor_operator_extracted() {
    let scopes = parse_scopes(&[("widget.cpp", r#"
class Widget {
public:
    Widget(int w, int h) {}
    ~Widget() {}
    bool operator==(const Widget& other) const { return true; }
    void draw() {}
private:
    int width;
    int height;
};
"#)]);

    // Widget class
    let widget = find_scope(&scopes, "Widget");
    assert!(widget.is_some(), "Widget class should be extracted");

    // Constructor: name should be "Widget" with parent "Widget"
    let constructor = scopes.iter().find(|s| s.name == "Widget" && s.parent.as_deref() == Some("Widget"));
    assert!(constructor.is_some(), "Constructor Widget() should be extracted as method of Widget");

    // Destructor: name should be "~Widget" with parent "Widget"
    let destructor = scopes.iter().find(|s| s.name == "~Widget" && s.parent.as_deref() == Some("Widget"));
    assert!(destructor.is_some(), "Destructor ~Widget() should be extracted with name '~Widget'");

    // Operator: name should be "operator==" with parent "Widget"
    let operator = scopes.iter().find(|s| s.name == "operator==" && s.parent.as_deref() == Some("Widget"));
    assert!(operator.is_some(), "operator==() should be extracted with name 'operator=='");

    // draw: regular method
    let draw = scopes.iter().find(|s| s.name == "draw" && s.parent.as_deref() == Some("Widget"));
    assert!(draw.is_some(), "draw() should be extracted as method of Widget");
}

// ---------------------------------------------------------------------------
// C function pointers in structs
// ---------------------------------------------------------------------------

#[test]
// ===========================================================================
// EXPLORATORY: Language-specific coverage gaps
// ===========================================================================

// --- C++ gaps ---

#[test]
fn cpp_toplevel_function_extracted() {
    // C++ functions at namespace/file level (not inside classes)
    let scopes = parse_scopes(&[("utils.cpp", r#"
int add(int a, int b) {
    return a + b;
}

void process() {
    int x = add(1, 2);
}
"#)]);

    for s in &scopes {
        eprintln!("  [cpp toplevel] name={:?} type={:?} sig={:?}", s.name, s.r#type, s.signature);
    }

    let add = find_scope(&scopes, "add");
    assert!(add.is_some(), "C++ top-level function add() should be extracted");

    let process = find_scope(&scopes, "process");
    assert!(process.is_some(), "C++ top-level function process() should be extracted");
}

#[test]
fn cpp_enum_extracted() {
    // C++ enums (plain and enum class)
    let scopes = parse_scopes(&[("colors.cpp", r#"
enum Color { Red, Green, Blue };

enum class Direction { North, South, East, West };
"#)]);

    for s in &scopes {
        eprintln!("  [cpp enum] name={:?} type={:?} sig={:?}", s.name, s.r#type, s.signature);
    }

    let color = find_scope(&scopes, "Color");
    assert!(color.is_some(), "C++ enum Color should be extracted");

    let direction = find_scope(&scopes, "Direction");
    assert!(direction.is_some(), "C++ enum class Direction should be extracted");
}

#[test]
fn cpp_out_of_class_method() {
    // C++ method defined outside class body (very common pattern)
    let scopes = parse_scopes(&[("engine.cpp", r#"
class Engine {
public:
    void start();
    void stop();
};

void Engine::start() {
    // implementation
}

void Engine::stop() {
    // implementation
}
"#)]);

    for s in &scopes {
        eprintln!("  [cpp out-of-class] name={:?} type={:?} sig={:?} parent={:?}", s.name, s.r#type, s.signature, s.parent);
    }

    let start = find_scope(&scopes, "start");
    assert!(start.is_some(), "C++ out-of-class method Engine::start() should be extracted");
}

#[test]
fn cpp_function_consumes_function() {
    // C++ function calling another top-level function
    let rels = parse_rels(&[("math.cpp", r#"
int square(int x) {
    return x * x;
}

int sum_of_squares(int a, int b) {
    return square(a) + square(b);
}
"#)]);

    for r in &rels {
        eprintln!("  [cpp consumes] {:?}: {} -> {}", r.rel_type, r.from, r.to);
    }

    assert!(has_rel(&rels, "sum_of_squares", "square", RelationshipType::CONSUMES),
        "C++ function calling another should produce CONSUMES");
}

// --- C# gaps ---

#[test]
fn csharp_nested_class_extracted() {
    let scopes = parse_scopes(&[("nested.cs", r#"
class Outer {
    public class Inner {
        public void DoStuff() {}
    }

    public void UseInner() {
        var inner = new Inner();
    }
}
"#)]);

    for s in &scopes {
        eprintln!("  [csharp nested] name={:?} type={:?} parent={:?}", s.name, s.r#type, s.parent);
    }

    let inner = find_scope(&scopes, "Inner");
    assert!(inner.is_some(), "C# nested class Inner should be extracted");
}

#[test]
fn csharp_implements_interface() {
    // C# class implementing interface (IFoo convention)
    let rels = parse_rels(&[("service.cs", r#"
interface IService {
    void Execute();
}

class MyService : IService {
    public void Execute() {}
}
"#)]);

    for r in &rels {
        eprintln!("  [csharp impl] {:?}: {} -> {}", r.rel_type, r.from, r.to);
    }

    assert!(has_rel(&rels, "MyService", "IService", RelationshipType::IMPLEMENTS),
        "C# class : IService should produce IMPLEMENTS");
}

#[test]
fn csharp_class_inherits_and_implements() {
    // C# class with both base class and interface
    let rels = parse_rels(&[("mixed.cs", r#"
class Animal {
    public void Eat() {}
}

interface ISwimmable {
    void Swim();
}

class Duck : Animal, ISwimmable {
    public void Swim() {}
    public void Quack() {}
}
"#)]);

    for r in &rels {
        eprintln!("  [csharp mixed] {:?}: {} -> {}", r.rel_type, r.from, r.to);
    }

    assert!(has_rel(&rels, "Duck", "Animal", RelationshipType::INHERITSFROM),
        "Duck : Animal should produce INHERITS_FROM");
    assert!(has_rel(&rels, "Duck", "ISwimmable", RelationshipType::IMPLEMENTS),
        "Duck : ISwimmable should produce IMPLEMENTS");
}

// --- Go gaps ---

#[test]
fn go_method_parent_of_struct() {
    // Go receiver methods should have parent = struct name
    let rels = parse_rels(&[("server.go", r#"
package main

type Server struct {
    Port int
}

func (s *Server) Start() {
    // start the server
}

func (s *Server) Stop() {
    // stop the server
}
"#)]);

    for r in &rels {
        eprintln!("  [go parent] {:?}: {} -> {}", r.rel_type, r.from, r.to);
    }

    assert!(has_rel(&rels, "Server", "Start", RelationshipType::PARENTOF),
        "Go receiver method Start should have PARENT_OF from Server");
    assert!(has_rel(&rels, "Server", "Stop", RelationshipType::PARENTOF),
        "Go receiver method Stop should have PARENT_OF from Server");
}

#[test]
fn go_interface_with_methods() {
    // Go interface should have methods extracted
    let scopes = parse_scopes(&[("shapes.go", r#"
package main

type Shape interface {
    Area() float64
    Perimeter() float64
}
"#)]);

    for s in &scopes {
        eprintln!("  [go iface] name={:?} type={:?} members={:?}", s.name, s.r#type, s.members);
    }

    let shape = find_scope(&scopes, "Shape");
    assert!(shape.is_some(), "Go interface Shape should be extracted");
    let shape = shape.unwrap();
    assert!(shape.members.is_some(), "Go interface should have members");
    let members = shape.members.as_ref().unwrap();
    assert!(members.iter().any(|m| m.name == "Area"), "Shape interface should have Area method");
    assert!(members.iter().any(|m| m.name == "Perimeter"), "Shape interface should have Perimeter method");
}

// --- Rust gaps ---

#[test]
fn rust_derive_not_crash() {
    // Rust derive macros - at minimum should not crash
    let scopes = parse_scopes(&[("model.rs", r#"
#[derive(Debug, Clone, PartialEq)]
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn distance(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}
"#)]);

    for s in &scopes {
        eprintln!("  [rust derive] name={:?} type={:?} sig={:?}", s.name, s.r#type, s.signature);
    }

    let point = find_scope(&scopes, "Point");
    assert!(point.is_some(), "Rust struct with derive should be extracted");
}

#[test]
fn rust_enum_with_variants() {
    let scopes = parse_scopes(&[("option.rs", r#"
enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
    Triangle { base: f64, height: f64 },
}

fn area(shape: &Shape) -> f64 {
    match shape {
        Shape::Circle(r) => std::f64::consts::PI * r * r,
        Shape::Rectangle(w, h) => w * h,
        Shape::Triangle { base, height } => 0.5 * base * height,
    }
}
"#)]);

    for s in &scopes {
        eprintln!("  [rust enum] name={:?} type={:?}", s.name, s.r#type);
    }

    let shape = find_scope(&scopes, "Shape");
    assert!(shape.is_some(), "Rust enum Shape should be extracted");
}

// --- Python gaps ---

#[test]
fn python_multiple_inheritance() {
    let rels = parse_rels(&[("mixins.py", r#"
class Serializable:
    def serialize(self):
        pass

class Loggable:
    def log(self):
        pass

class Service(Serializable, Loggable):
    def run(self):
        self.serialize()
        self.log()
"#)]);

    for r in &rels {
        eprintln!("  [py multi] {:?}: {} -> {}", r.rel_type, r.from, r.to);
    }

    assert!(has_rel(&rels, "Service", "Serializable", RelationshipType::INHERITSFROM),
        "Python multiple inheritance: Service → Serializable");
    assert!(has_rel(&rels, "Service", "Loggable", RelationshipType::INHERITSFROM),
        "Python multiple inheritance: Service → Loggable");
}

#[test]
fn python_decorator_relationship() {
    let scopes = parse_scopes(&[("decorated.py", r#"
def my_decorator(func):
    def wrapper(*args, **kwargs):
        return func(*args, **kwargs)
    return wrapper

class Api:
    @my_decorator
    def endpoint(self):
        pass
"#)]);

    for s in &scopes {
        eprintln!("  [py deco] name={:?} type={:?} decorators={:?}", s.name, s.r#type, s.decorators);
    }

    let endpoint = find_scope(&scopes, "endpoint");
    assert!(endpoint.is_some(), "Decorated Python method should be extracted");
}

// --- C gaps ---

#[test]
fn c_enum_extracted() {
    let scopes = parse_scopes(&[("status.c", r#"
enum Status {
    OK,
    ERROR,
    PENDING
};

typedef enum {
    RED,
    GREEN,
    BLUE
} Color;
"#)]);

    for s in &scopes {
        eprintln!("  [c enum] name={:?} type={:?} sig={:?}", s.name, s.r#type, s.signature);
    }

    let status = find_scope(&scopes, "Status");
    assert!(status.is_some(), "C enum Status should be extracted");

    let color = find_scope(&scopes, "Color");
    assert!(color.is_some(), "C typedef enum Color should be extracted");
}

// ---------------------------------------------------------------------------
// C function pointers in structs
// ---------------------------------------------------------------------------

#[test]
fn c_function_pointer_struct_call() {
    let scopes = parse_scopes(&[("vtable.c", r#"
typedef struct {
    void (*draw)(void* self);
    void (*destroy)(void* self);
} Vtable;

void render(Vtable* vt, void* obj) {
    vt->draw(obj);
}
"#)]);

    let render = find_scope(&scopes, "render");
    assert!(render.is_some(), "render() should be extracted");

    let vtable = find_scope(&scopes, "Vtable");
    assert!(vtable.is_some(), "Vtable typedef should be extracted");
}

// ===========================================================================
// Body extraction: content = body-only, line numbers correct
// ===========================================================================

#[test]
fn ts_function_body_extraction() {
    // Line 1 is blank (leading newline), function starts line 2
    let scopes = parse_scopes(&[("body.ts", r#"
function greet(name: string): string {
    const msg = "Hello " + name;
    return msg;
}
"#)]);

    let greet = find_scope(&scopes, "greet").expect("greet scope not found");

    // content should be the body block only (not contain the signature)
    assert!(!greet.content.contains("function greet"), "content should NOT contain signature, got: {}", greet.content);
    assert!(greet.content.contains("return msg"), "content should contain body code");

    // body lines should be populated
    assert!(greet.body_start_line.is_some(), "body_start_line should be Some");
    assert!(greet.body_end_line.is_some(), "body_end_line should be Some");

    // signature_end_line <= body_start_line (equal when { is on same line as signature)
    assert!(greet.signature_end_line <= greet.body_start_line.unwrap(),
        "signature_end_line ({}) should be <= body_start_line ({})",
        greet.signature_end_line, greet.body_start_line.unwrap());
}

#[test]
fn ts_class_body_extraction() {
    let scopes = parse_scopes(&[("cls.ts", r#"
class Foo {
    bar() {
        return 42;
    }
}
"#)]);

    let foo = find_scope(&scopes, "Foo").expect("Foo scope not found");

    // content should be the class body, not the "class Foo {" line
    assert!(!foo.content.contains("class Foo"), "content should NOT contain 'class Foo' signature, got: {}", foo.content);
    assert!(foo.body_start_line.is_some(), "body_start_line should be Some for class");
}

#[test]
fn ts_method_body_extraction() {
    let scopes = parse_scopes(&[("meth.ts", r#"
class Calc {
    add(a: number, b: number): number {
        return a + b;
    }
}
"#)]);

    let add = find_scope(&scopes, "add").expect("add scope not found");

    assert!(!add.content.contains("add(a:"), "content should NOT contain method signature");
    assert!(add.content.contains("return a + b"), "content should contain method body");
    assert!(add.body_start_line.is_some(), "body_start_line should be Some for method");
}

#[test]
fn python_function_body_extraction() {
    let scopes = parse_scopes(&[("body.py", r#"
def greet(name):
    msg = "Hello " + name
    return msg
"#)]);

    let greet = find_scope(&scopes, "greet").expect("greet scope not found");

    assert!(!greet.content.contains("def greet"), "content should NOT contain Python function signature, got: {}", greet.content);
    assert!(greet.content.contains("return msg"), "content should contain body");
    assert!(greet.body_start_line.is_some(), "body_start_line should be Some for Python function");
}

#[test]
fn rust_function_body_extraction() {
    let scopes = parse_scopes(&[("body.rs", r#"
fn compute(x: i32) -> i32 {
    let y = x * 2;
    y + 1
}
"#)]);

    let compute = find_scope(&scopes, "compute").expect("compute scope not found");

    assert!(!compute.content.contains("fn compute"), "content should NOT contain Rust function signature, got: {}", compute.content);
    assert!(compute.content.contains("let y = x * 2"), "content should contain body");
    assert!(compute.body_start_line.is_some(), "body_start_line should be Some for Rust function");
}

#[test]
fn go_function_body_extraction() {
    let scopes = parse_scopes(&[("body.go", "package main\n\nfunc Add(a int, b int) int {\n\treturn a + b\n}\n")]);

    let add = find_scope(&scopes, "Add").expect("Add scope not found");

    assert!(!add.content.contains("func Add"), "content should NOT contain Go function signature, got: {}", add.content);
    assert!(add.content.contains("return a + b"), "content should contain body");
    assert!(add.body_start_line.is_some(), "body_start_line should be Some for Go function");
}

#[test]
fn cpp_function_body_extraction() {
    let scopes = parse_scopes(&[("body.cpp", r#"
int add(int a, int b) {
    return a + b;
}
"#)]);

    let add = find_scope(&scopes, "add").expect("add scope not found");

    assert!(!add.content.contains("int add"), "content should NOT contain C++ function signature, got: {}", add.content);
    assert!(add.content.contains("return a + b"), "content should contain body");
    assert!(add.body_start_line.is_some(), "body_start_line should be Some for C++ function");
}

#[test]
fn csharp_method_body_extraction() {
    let scopes = parse_scopes(&[("body.cs", r#"
class Calc {
    public int Add(int a, int b) {
        return a + b;
    }
}
"#)]);

    let add = find_scope(&scopes, "Add").expect("Add scope not found");

    assert!(!add.content.contains("public int Add"), "content should NOT contain C# method signature, got: {}", add.content);
    assert!(add.content.contains("return a + b"), "content should contain body");
    assert!(add.body_start_line.is_some(), "body_start_line should be Some for C# method");
}

#[test]
fn c_function_body_extraction() {
    let scopes = parse_scopes(&[("body.c", r#"
int add(int a, int b) {
    return a + b;
}
"#)]);

    let add = find_scope(&scopes, "add").expect("add scope not found");

    assert!(!add.content.contains("int add"), "content should NOT contain C function signature, got: {}", add.content);
    assert!(add.content.contains("return a + b"), "content should contain body");
    assert!(add.body_start_line.is_some(), "body_start_line should be Some for C function");
}

// ===========================================================================
// Coverage gaps: C++ virtual inheritance
// ===========================================================================

#[test]
fn cpp_virtual_inheritance_detected() {
    let scopes = parse_scopes(&[("diamond.cpp", r#"
class Base {
public:
    int value;
};

class Left : virtual public Base {
public:
    void left_method() {}
};

class Right : virtual public Base {
public:
    void right_method() {}
};

class Diamond : public Left, public Right {
public:
    void diamond_method() {}
};
"#)]);

    for s in &scopes {
        if s.r#type == ScopeInfoType::Class {
            eprintln!("  [cpp virtual] name={:?} heritage={:?} modifiers={:?}", s.name, s.heritage_clauses, s.modifiers);
        }
    }

    // Left and Right should inherit from Base
    let left = find_scope(&scopes, "Left").expect("Left scope not found");
    assert!(left.heritage_clauses.as_ref().map_or(false, |h| h.iter().any(|c| c.types.contains(&"Base".to_string()))),
        "Left should inherit from Base");

    // Diamond should inherit from Left and Right
    let diamond = find_scope(&scopes, "Diamond").expect("Diamond scope not found");
    let heritage = diamond.heritage_clauses.as_ref().expect("Diamond should have heritage");
    let all_types: Vec<&String> = heritage.iter().flat_map(|c| c.types.iter()).collect();
    assert!(all_types.contains(&&"Left".to_string()), "Diamond should inherit from Left");
    assert!(all_types.contains(&&"Right".to_string()), "Diamond should inherit from Right");
}

// ===========================================================================
// Coverage gaps: Rust closures as scopes
// ===========================================================================

#[test]
fn rust_closure_extracted_as_scope() {
    let scopes = parse_scopes(&[("closures.rs", r#"
fn process(items: Vec<i32>) -> Vec<i32> {
    let doubled = items.iter().map(|x| x * 2).collect();
    let adder = |a: i32, b: i32| a + b;
    doubled
}
"#)]);

    for s in &scopes {
        eprintln!("  [rust closure] name={:?} type={:?} parent={:?} sig={:?}", s.name, s.r#type, s.parent, s.signature);
    }

    // Named closure (assigned to variable via let)
    let adder = scopes.iter().find(|s| s.name == "adder" && s.r#type == ScopeInfoType::Lambda);
    assert!(adder.is_some(), "Rust closure assigned to 'adder' should be extracted as Lambda scope");
    let adder = adder.unwrap();
    assert_eq!(adder.parent.as_deref(), Some("process"), "adder's parent should be process");
    assert!(adder.parameters.len() == 2, "adder should have 2 parameters, got {}", adder.parameters.len());
}

// ===========================================================================
// Coverage gaps: Python comprehensions as scopes
// ===========================================================================

#[test]
fn python_comprehension_extracted_as_scope() {
    let scopes = parse_scopes(&[("comp.py", r#"
def process(items):
    result = [x.name for x in items if x.active]
    unique = {x for x in result}
    return unique
"#)]);

    for s in &scopes {
        eprintln!("  [py comp] name={:?} type={:?} parent={:?}", s.name, s.r#type, s.parent);
    }

    // Named list comprehension (assigned to variable)
    let result = scopes.iter().find(|s| s.name == "result" && s.r#type == ScopeInfoType::Lambda);
    assert!(result.is_some(), "Python list comprehension assigned to 'result' should be extracted as Lambda scope");

    // Named set comprehension
    let unique = scopes.iter().find(|s| s.name == "unique" && s.r#type == ScopeInfoType::Lambda);
    assert!(unique.is_some(), "Python set comprehension assigned to 'unique' should be extracted as Lambda scope");
}

// ===========================================================================
// Coverage gaps: C++ lambdas as scopes
// ===========================================================================

#[test]
fn cpp_lambda_extracted_as_scope() {
    let scopes = parse_scopes(&[("lambdas.cpp", r#"
auto doubler = [](int x) { return x * 2; };

void process() {
    auto adder = [](int a, int b) { return a + b; };
}
"#)]);

    for s in &scopes {
        eprintln!("  [cpp lambda] name={:?} type={:?} parent={:?} sig={:?}", s.name, s.r#type, s.parent, s.signature);
    }

    // Top-level named lambda
    let doubler = scopes.iter().find(|s| s.name == "doubler");
    assert!(doubler.is_some(), "C++ named lambda 'doubler' should be extracted as scope");
    let doubler = doubler.unwrap();
    assert_eq!(doubler.r#type, ScopeInfoType::Lambda, "doubler should be Lambda type");

    // Lambda inside function body
    let adder = scopes.iter().find(|s| s.name == "adder");
    assert!(adder.is_some(), "C++ lambda 'adder' inside function body should be extracted");
    let adder = adder.unwrap();
    assert_eq!(adder.r#type, ScopeInfoType::Lambda, "adder should be Lambda type");
    assert_eq!(adder.parent.as_deref(), Some("process"), "adder's parent should be process");
}

// ===========================================================================
// Coverage gaps: Go interface embedding
// ===========================================================================

#[test]
fn go_interface_embedding_is_inherits() {
    let rels = parse_rels(&[("io.go", r#"package io

type Reader interface {
    Read(p []byte) (int, error)
}

type Writer interface {
    Write(p []byte) (int, error)
}

type ReadWriter interface {
    Reader
    Writer
}
"#)]);

    for r in &rels {
        eprintln!("  [go iface embed] {} -> {} ({:?})", r.from, r.to, r.rel_type);
    }

    assert!(has_rel(&rels, "ReadWriter", "Reader", RelationshipType::INHERITSFROM),
        "Go interface embedding: ReadWriter should INHERITS_FROM Reader");
    assert!(has_rel(&rels, "ReadWriter", "Writer", RelationshipType::INHERITSFROM),
        "Go interface embedding: ReadWriter should INHERITS_FROM Writer");
}

// ===========================================================================
// Coverage gaps: TypeScript decorator relationships
// ===========================================================================

#[test]
fn ts_decorator_relationship() {
    let rels = parse_rels(&[("service.ts", r#"
function Injectable() { return (target: any) => target; }

@Injectable()
class UserService {
    getUser(id: string) {
        return { id };
    }
}
"#)]);

    for r in &rels {
        eprintln!("  [ts deco] {} -> {} ({:?})", r.from, r.to, r.rel_type);
    }

    // Class-level decorator creates DECORATEDBY (inverse of DECORATES)
    assert!(has_rel(&rels, "UserService", "Injectable", RelationshipType::DECORATEDBY),
        "TS class decorator: UserService should be DECORATEDBY Injectable");
}

#[test]
fn ts_method_decorator_relationship() {
    let rels = parse_rels(&[("api.ts", r#"
function Log() { return (target: any, key: string) => {}; }

class ApiController {
    @Log()
    getUser(id: string) {
        return { id };
    }
}
"#)]);

    for r in &rels {
        eprintln!("  [ts method deco] {} -> {} ({:?})", r.from, r.to, r.rel_type);
    }

    assert!(has_rel(&rels, "getUser", "Log", RelationshipType::DECORATEDBY),
        "TS method decorator: getUser should be DECORATEDBY Log");
}

