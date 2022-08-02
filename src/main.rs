mod devices;

use devices::HelloTriangleApplication;

fn main() {
    let app = HelloTriangleApplication::new();
    app.run();
}
