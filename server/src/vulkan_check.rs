use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::VulkanLibrary;
use tracing::info;

pub fn verify_vulkan_gpu() -> bool {
    info!("--- VULKAN LOG: Initializing GPU Verification ---");
    
    let library = match VulkanLibrary::new() {
        Ok(l) => l,
        Err(e) => {
            info!("--- VULKAN LOG: Failed to load Vulkan library: {} ---", e);
            return false;
        }
    };

    let instance = match Instance::new(library, InstanceCreateInfo::default()) {
        Ok(i) => i,
        Err(e) => {
            info!("--- VULKAN LOG: Failed to create instance: {} ---", e);
            return false;
        }
    };

    let physical_devices = match instance.enumerate_physical_devices() {
        Ok(devices) => devices,
        Err(e) => {
            info!("--- VULKAN LOG: Failed to enumerate physical devices: {} ---", e);
            return false;
        }
    };

    let mut found_gpu = false;
    for device in physical_devices {
        let properties = device.properties();
        let name = &properties.device_name;
        let device_type = properties.device_type;
        
        if device_type == PhysicalDeviceType::DiscreteGpu || device_type == PhysicalDeviceType::IntegratedGpu {
            info!("--- VULKAN LOG: Found GPU: {} ({:?}) ---", name, device_type);
            found_gpu = true;
            
            // Check for floating point compute support
            if device.queue_family_properties().iter().any(|q| q.queue_flags.contains(vulkano::device::QueueFlags::COMPUTE)) {
                info!("--- VULKAN LOG: GPU {} supports Floating Point Compute via Vulkan ---", name);
            } else {
                info!("--- VULKAN LOG: GPU {} DOES NOT support Compute ---", name);
            }
        }
    }

    if !found_gpu {
        info!("--- VULKAN LOG: No Integrated or Discrete GPU found via Vulkan ---");
    }

    found_gpu
}
