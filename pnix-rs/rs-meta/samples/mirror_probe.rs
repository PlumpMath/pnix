// stage3 mirror probe — representative subset Rust, compiled by rustc and
// evaluated at stage1(native)/stage2/stage2' for the canonical AST + output
// mirror. Expected stdout: "29 62".
struct Point {
    x: i64,
    y: i64,
}

enum Shape {
    Circle(i64),
    Rect { w: i64, h: i64 },
}

impl Point {
    fn new(x: i64, y: i64) -> Point {
        Point { x, y }
    }
    fn sum(&self) -> i64 {
        self.x + self.y
    }
}

fn area(s: Shape) -> i64 {
    match s {
        Shape::Circle(r) => 3 * r * r,
        Shape::Rect { w, h } => w * h,
    }
}

fn main() {
    let p = Point::new(3, 4);
    let mut total = p.sum();
    total += area(Shape::Rect { w: 2, h: 5 });
    total += area(Shape::Circle(2));
    let mut v = Vec::new();
    let mut i = 0;
    while i < 4 {
        v.push(total + i);
        i += 1;
    }
    let mut acc = 0;
    for x in &v {
        if *x % 2 == 0 {
            acc += *x;
        }
    }
    println!("{} {}", total, acc);
}
