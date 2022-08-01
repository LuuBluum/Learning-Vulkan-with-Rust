mod instance;

use instance::HelloTriangleApplication;

fn main() {
    let app = HelloTriangleApplication::new();
    app.run();
}
