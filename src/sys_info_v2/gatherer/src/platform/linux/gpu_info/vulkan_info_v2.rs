use crate::logging::message;

pub struct VulkanInfo {
    _entry: ash::Entry,
    vk_instance: ash::Instance,
}

impl Drop for VulkanInfo {
    fn drop(&mut self) {
        unsafe {
            self.vk_instance.destroy_instance(None);
        }
    }
}

impl VulkanInfo {
    pub fn new() -> Option<Self> {
        use crate::{critical, message};
        use ash::{vk, Entry};

        return None;

        message!("Gatherer::VkInfo", "Loading Vulkan library");
        let _entry = match unsafe { Entry::load() } {
            Ok(e) => e,
            Err(e) => {
                critical!(
                    "Gatherer::GPU",
                    "Failed to get Vulkan information: Could not load 'libvulkan.so.1'; {}",
                    e
                );
                return None;
            }
        };
        message!("Gatherer::VkInfo", "Loaded Vulkan library");

        let app_info = vk::ApplicationInfo {
            api_version: vk::make_api_version(0, 1, 0, 0),
            ..Default::default()
        };
        let create_info = vk::InstanceCreateInfo {
            p_application_info: &app_info,
            ..Default::default()
        };

        message!("Gatherer::VkInfo", "Creating Vulkan instance");
        let instance = match unsafe { _entry.create_instance(&create_info, None) } {
            Ok(i) => i,
            Err(e) => {
                critical!(
                    "Gatherer::GPU",
                    "Failed to get Vulkan information: Could not create instance; {}",
                    e
                );
                return None;
            }
        };
        message!("Gatherer::VkInfo", "Created Vulkan instance");

        Some(Self {
            _entry: _entry,
            vk_instance: instance,
        })
    }

    pub unsafe fn supported_vulkan_versions(
        &self,
    ) -> Option<std::collections::HashMap<u32, (u16, u16, u16)>> {
        None
    }
}
