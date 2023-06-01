#include <vulkan/vulkan.h>
#include <stdio.h>

int main()
{
    fprintf(stderr, "%d %d\n", sizeof(VkPhysicalDeviceLimits), sizeof(VkPhysicalDeviceSparseProperties));
    return 0;
}

