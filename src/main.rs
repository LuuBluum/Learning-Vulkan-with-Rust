mod basecode;

use basecode::HelloTriangleApplication;

fn main() {
    let app = HelloTriangleApplication::new();
    app.run().unwrap();
}
