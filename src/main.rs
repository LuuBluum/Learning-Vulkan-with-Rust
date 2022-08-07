mod swapchain;

use swapchain::HelloTriangleApplication;

fn main() {
    let app = HelloTriangleApplication::new();
    app.run();
}
