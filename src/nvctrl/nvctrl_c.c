#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <X11/Xlib.h>
#include <NVCtrl/NVCtrl.h>
#include <NVCtrl/NVCtrlLib.h>

Display *disp;

int nv_init(void) {
    if(disp != NULL) {
        XCloseDisplay(disp);
    }
    disp = XOpenDisplay(NULL);
    if(disp == NULL) {
        return 0;
    }
    return 1;
}

int nv_deinit(void) {
    if(disp == NULL) {
        return 0;
    }

    XCloseDisplay(disp);
    disp = NULL;
    return 1;
}

int nv_get_temp(int *val) {


    if(!XNVCTRLQueryAttribute(disp, 0, 0,
        NV_CTRL_GPU_CORE_TEMPERATURE, val)){

        fprintf(stderr, "Cannot get temperature attribute");
        return 0;
    }

    return 1;

}

int nv_get_ctrl_status(int *val) {

    if(!XNVCTRLQueryTargetAttribute(disp, NV_CTRL_TARGET_TYPE_GPU, 0, 0,
        NV_CTRL_GPU_COOLER_MANUAL_CONTROL, val)){

        fprintf(stderr, "Cannot get control status");
        return 0;
    }

    return 1;
}

int nv_get_fanspeed(int *val) {

    if(!XNVCTRLQueryTargetAttribute(disp, NV_CTRL_TARGET_TYPE_COOLER, 0, 0,
        NV_CTRL_THERMAL_COOLER_CURRENT_LEVEL, val)){

        fprintf(stderr, "Cannot get fanspeed");
        return 0;
    }

    return 1;
}

int nv_get_fanspeed_rpm(int *val) {
    if(!XNVCTRLQueryTargetAttribute(disp, NV_CTRL_TARGET_TYPE_COOLER, 0, 0,
        NV_CTRL_THERMAL_COOLER_SPEED, val)){

        fprintf(stderr, "Cannot get fanspeed (rpm)");
        return 0;
    }

    return 1;
}

int nv_set_ctrl_type(int val) {

    return XNVCTRLSetTargetAttributeAndGetStatus(
            disp, NV_CTRL_TARGET_TYPE_GPU, 0, 0,
            NV_CTRL_GPU_COOLER_MANUAL_CONTROL, val);

}

int nv_set_fanspeed(int val) {
    return XNVCTRLSetTargetAttributeAndGetStatus(
            disp, NV_CTRL_TARGET_TYPE_COOLER, 0, 0,
            NV_CTRL_THERMAL_COOLER_LEVEL, val);
}

int nv_get_version(char **ptr) {
    return XNVCTRLQueryStringAttribute(
            disp, 0, 0, NV_CTRL_STRING_NVIDIA_DRIVER_VERSION, ptr);
}

int nv_get_utilization(char **ptr) {
    return XNVCTRLQueryTargetStringAttribute(
            disp, NV_CTRL_TARGET_TYPE_GPU, 0, 0,
            NV_CTRL_STRING_GPU_UTILIZATION, ptr);
}

int nv_get_adapter(char **ptr) {
    return XNVCTRLQueryTargetStringAttribute(
            disp, NV_CTRL_TARGET_TYPE_GPU, 0, 0,
            NV_CTRL_STRING_PRODUCT_NAME, ptr);
}

/*
 * vim:ts=4:sw=4:et
 */
