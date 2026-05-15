//! JNI/NDK bridge for `spectralfilm-core`.
//!
//! This crate exposes a small, stable Android-facing ABI:
//! - Java/Kotlin JNI entry points for `com.latentcam.spectralfilm.SpectralEngine`.
//! - C ABI functions for direct native integrations or custom Java wrappers.

use std::ffi::{c_char, CStr, CString};
use std::ptr;
use std::sync::Once;

use jni::objects::{JByteBuffer, JClass, JFloatArray, JString};
use jni::sys::{jboolean, jdouble, jfloatArray, jint, jlong, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use log::{error, LevelFilter};
use spectralfilm_core::profiles::profile_from_json;
use spectralfilm_core::runtime::params::{CameraParamsPatch, EnlargerParamsPatch, RuntimeParamsPatch, RuntimePhotoParams};
use spectralfilm_core::runtime::params_builder::digest_params;
use spectralfilm_core::runtime::pipeline::SimulationPipeline;

static LOGGER: Once = Once::new();

fn init_logger() {
    LOGGER.call_once(|| {
        #[cfg(target_os = "android")]
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(LevelFilter::Info)
                .with_tag("SpectralFilm"),
        );
        #[cfg(not(target_os = "android"))]
        let _ = LevelFilter::Info;
    });
}

pub struct Engine {
    params: RuntimePhotoParams,
    pipeline: SimulationPipeline,
    last_width: usize,
    last_height: usize,
    last_channels: usize,
    last_error: Option<String>,
}

impl Engine {
    fn new(film_json: &str, print_json: &str) -> Result<Self, String> {
        let film = profile_from_json(film_json).map_err(|e| e.to_string())?;
        let print = profile_from_json(print_json).map_err(|e| e.to_string())?;
        let params = digest_params(RuntimePhotoParams::new(film, print));
        Ok(Self::from_params(params))
    }

    fn from_params(params: RuntimePhotoParams) -> Self {
        Self {
            pipeline: SimulationPipeline::new(params.clone()),
            params,
            last_width: 0,
            last_height: 0,
            last_channels: 3,
            last_error: None,
        }
    }

    fn from_params_json(params_json: &str) -> Result<Self, String> {
        let params = RuntimePhotoParams::from_json(params_json).map_err(|e| e.to_string())?;
        Ok(Self::from_params(digest_params(params)))
    }

    fn update_params(&mut self, params: RuntimePhotoParams) {
        self.pipeline.update(params.clone());
        self.params = params;
        self.last_error = None;
    }

    fn update_params_json(&mut self, params_json: &str) -> Result<(), String> {
        let params = RuntimePhotoParams::from_json(params_json).map_err(|e| e.to_string())?;
        self.update_params(digest_params(params));
        Ok(())
    }

    fn soft_update_params_json(&mut self, patch_json: &str) -> Result<(), String> {
        let patch = RuntimeParamsPatch::from_json(patch_json).map_err(|e| e.to_string())?;
        self.soft_update_params(patch);
        Ok(())
    }

    fn soft_update_params(&mut self, patch: RuntimeParamsPatch) {
        let mut params = self.params.clone();
        params.apply_patch(patch);
        self.update_params(digest_params(params));
    }

    fn soft_update_exposure_print_filters(
        &mut self,
        exposure_compensation_ev: f64,
        auto_exposure_mode: jint,
        print_exposure: f64,
        y_filter_shift: f64,
        m_filter_shift: f64,
        c_filter_neutral: f64,
        y_filter_neutral: f64,
        m_filter_neutral: f64,
    ) {
        let mut patch = RuntimeParamsPatch::default();
        if exposure_compensation_ev.is_finite() || auto_exposure_mode >= 0 {
            let mut camera = CameraParamsPatch::default();
            if exposure_compensation_ev.is_finite() {
                camera.exposure_compensation_ev = Some(exposure_compensation_ev);
            }
            if auto_exposure_mode >= 0 {
                camera.auto_exposure = Some(auto_exposure_mode != 0);
            }
            patch.camera = Some(camera);
        }
        if print_exposure.is_finite()
            || y_filter_shift.is_finite()
            || m_filter_shift.is_finite()
            || c_filter_neutral.is_finite()
            || y_filter_neutral.is_finite()
            || m_filter_neutral.is_finite()
        {
            let mut enlarger = EnlargerParamsPatch::default();
            if print_exposure.is_finite() { enlarger.print_exposure = Some(print_exposure); }
            if y_filter_shift.is_finite() { enlarger.y_filter_shift = Some(y_filter_shift); }
            if m_filter_shift.is_finite() { enlarger.m_filter_shift = Some(m_filter_shift); }
            if c_filter_neutral.is_finite() { enlarger.c_filter_neutral = Some(c_filter_neutral); }
            if y_filter_neutral.is_finite() { enlarger.y_filter_neutral = Some(y_filter_neutral); }
            if m_filter_neutral.is_finite() { enlarger.m_filter_neutral = Some(m_filter_neutral); }
            patch.enlarger = Some(enlarger);
        }
        self.soft_update_params(patch);
    }

    fn process_f32(&mut self, input: &[f32], width: usize, height: usize) -> Result<Vec<f32>, String> {
        let input64: Vec<f64> = input.iter().map(|&v| v as f64).collect();
        let out = self.pipeline.process(&input64, width, height).map_err(|e| e.to_string())?;
        self.last_width = out.width;
        self.last_height = out.height;
        self.last_channels = out.channels;
        Ok(out.data.into_iter().map(|v| v as f32).collect())
    }

    fn process_direct_f32(&mut self, input: &[f32], output: &mut [f32], width: usize, height: usize) -> Result<(), String> {
        let expected = width * height * 3;
        if input.len() < expected || output.len() < expected {
            return Err(format!("Direct buffer length too small. Expected at least {expected} floats."));
        }
        
        let input64: Vec<f64> = input[..expected].iter().map(|&v| v as f64).collect();
        let out = self.pipeline.process(&input64, width, height).map_err(|e| e.to_string())?;
        
        for (i, &val) in out.data.iter().enumerate() {
            output[i] = val as f32;
        }
        
        self.last_width = out.width;
        self.last_height = out.height;
        self.last_channels = out.channels;
        Ok(())
    }

    fn generate_lut_f32(&mut self, lut_size: usize) -> Result<Vec<f32>, String> {
        self.pipeline.generate_lut(lut_size).map_err(|e| e.to_string())
    }
}

unsafe fn engine_mut(handle: jlong) -> Option<&'static mut Engine> {
    if handle == 0 { None } else { (handle as *mut Engine).as_mut() }
}

fn throw(env: &mut JNIEnv, msg: impl AsRef<str>) {
    let _ = env.throw_new("java/lang/RuntimeException", msg.as_ref());
}

fn get_string(env: &mut JNIEnv, s: &JString) -> Result<String, String> {
    env.get_string(s).map(|v| v.into()).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// JNI API for Kotlin/Java wrapper class:
// package com.latentcam.spectralfilm
// class SpectralEngine { external fun nativeCreate(...): Long ... }
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "system" fn Java_com_latentcam_spectralfilm_SpectralEngine_nativeInitLogger(
    _env: JNIEnv,
    _class: JClass,
) {
    init_logger();
}

#[no_mangle]
pub extern "system" fn Java_com_latentcam_spectralfilm_SpectralEngine_nativeCreate(
    mut env: JNIEnv,
    _class: JClass,
    film_json: JString,
    print_json: JString,
) -> jlong {
    init_logger();
    let film = match get_string(&mut env, &film_json) {
        Ok(s) => s,
        Err(e) => { throw(&mut env, e); return 0; }
    };
    let print = match get_string(&mut env, &print_json) {
        Ok(s) => s,
        Err(e) => { throw(&mut env, e); return 0; }
    };
    match Engine::new(&film, &print) {
        Ok(engine) => Box::into_raw(Box::new(engine)) as jlong,
        Err(e) => { throw(&mut env, e); 0 }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_latentcam_spectralfilm_SpectralEngine_nativeCreateFromParamsJson(
    mut env: JNIEnv,
    _class: JClass,
    params_json: JString,
) -> jlong {
    init_logger();
    let params = match get_string(&mut env, &params_json) {
        Ok(s) => s,
        Err(e) => { throw(&mut env, e); return 0; }
    };
    match Engine::from_params_json(&params) {
        Ok(engine) => Box::into_raw(Box::new(engine)) as jlong,
        Err(e) => { throw(&mut env, e); 0 }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_latentcam_spectralfilm_SpectralEngine_nativeRelease(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle != 0 {
        unsafe { drop(Box::from_raw(handle as *mut Engine)); }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_latentcam_spectralfilm_SpectralEngine_nativeProcess(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    input: JFloatArray,
    width: jint,
    height: jint,
) -> jfloatArray {
    init_logger();
    let Some(engine) = (unsafe { engine_mut(handle) }) else {
        throw(&mut env, "Invalid native engine handle");
        return ptr::null_mut();
    };
    if width <= 0 || height <= 0 {
        throw(&mut env, "width and height must be positive");
        return ptr::null_mut();
    }
    let len = match env.get_array_length(&input) {
        Ok(v) => v as usize,
        Err(e) => { throw(&mut env, e.to_string()); return ptr::null_mut(); }
    };
    let expected = width as usize * height as usize * 3;
    if len < expected {
        throw(&mut env, format!("input array length {len} is smaller than width*height*3 ({expected})"));
        return ptr::null_mut();
    }
    let mut buf = vec![0.0f32; expected];
    if let Err(e) = env.get_float_array_region(&input, 0, &mut buf) {
        throw(&mut env, e.to_string());
        return ptr::null_mut();
    }
    let result = match engine.process_f32(&buf, width as usize, height as usize) {
        Ok(v) => v,
        Err(e) => {
            engine.last_error = Some(e.clone());
            throw(&mut env, e);
            return ptr::null_mut();
        }
    };
    let out = match env.new_float_array(result.len() as i32) {
        Ok(a) => a,
        Err(e) => { throw(&mut env, e.to_string()); return ptr::null_mut(); }
    };
    if let Err(e) = env.set_float_array_region(&out, 0, &result) {
        throw(&mut env, e.to_string());
        return ptr::null_mut();
    }
    out.into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_latentcam_spectralfilm_SpectralEngine_nativeGenerateLut(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    lut_size: jint,
) -> jfloatArray {
    init_logger();
    let Some(engine) = (unsafe { engine_mut(handle) }) else {
        throw(&mut env, "Invalid native engine handle");
        return ptr::null_mut();
    };
    if lut_size < 2 || lut_size > 64 {
        throw(&mut env, "lut_size must be between 2 and 64");
        return ptr::null_mut();
    }
    
    let result = match engine.generate_lut_f32(lut_size as usize) {
        Ok(v) => v,
        Err(e) => {
            engine.last_error = Some(e.clone());
            throw(&mut env, e);
            return ptr::null_mut();
        }
    };
    
    let out = match env.new_float_array(result.len() as i32) {
        Ok(a) => a,
        Err(e) => { throw(&mut env, e.to_string()); return ptr::null_mut(); }
    };
    if let Err(e) = env.set_float_array_region(&out, 0, &result) {
        throw(&mut env, e.to_string());
        return ptr::null_mut();
    }
    out.into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_latentcam_spectralfilm_SpectralEngine_nativeProcessDirect(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    input_buf: JByteBuffer,
    output_buf: JByteBuffer,
    width: jint,
    height: jint,
) -> jboolean {
    init_logger();
    let Some(engine) = (unsafe { engine_mut(handle) }) else {
        throw(&mut env, "Invalid native engine handle");
        return JNI_FALSE;
    };
    if width <= 0 || height <= 0 {
        throw(&mut env, "width and height must be positive");
        return JNI_FALSE;
    }
    
    // Get direct byte buffers
    let in_ptr = match env.get_direct_buffer_address(&input_buf) {
        Ok(addr) => addr,
        Err(e) => { throw(&mut env, e.to_string()); return JNI_FALSE; }
    };
    let in_cap = match env.get_direct_buffer_capacity(&input_buf) {
        Ok(cap) => cap,
        Err(e) => { throw(&mut env, e.to_string()); return JNI_FALSE; }
    };
    let out_ptr = match env.get_direct_buffer_address(&output_buf) {
        Ok(addr) => addr,
        Err(e) => { throw(&mut env, e.to_string()); return JNI_FALSE; }
    };
    let out_cap = match env.get_direct_buffer_capacity(&output_buf) {
        Ok(cap) => cap,
        Err(e) => { throw(&mut env, e.to_string()); return JNI_FALSE; }
    };
    
    let expected_floats = width as usize * height as usize * 3;
    let expected_bytes = expected_floats * 4;
    
    if in_cap < expected_bytes || out_cap < expected_bytes {
        throw(&mut env, format!("Direct buffer too small. Expected {expected_bytes} bytes."));
        return JNI_FALSE;
    }
    
    let in_f32 = unsafe { std::slice::from_raw_parts(in_ptr as *const f32, expected_floats) };
    let out_f32 = unsafe { std::slice::from_raw_parts_mut(out_ptr as *mut f32, expected_floats) };
    
    match engine.process_direct_f32(in_f32, out_f32, width as usize, height as usize) {
        Ok(_) => JNI_TRUE,
        Err(e) => {
            engine.last_error = Some(e.clone());
            throw(&mut env, e);
            JNI_FALSE
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_latentcam_spectralfilm_SpectralEngine_nativeUpdateParamsJson(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    params_json: JString,
) -> jboolean {
    init_logger();
    let Some(engine) = (unsafe { engine_mut(handle) }) else {
        throw(&mut env, "Invalid native engine handle");
        return JNI_FALSE;
    };
    let params = match get_string(&mut env, &params_json) {
        Ok(s) => s,
        Err(e) => { throw(&mut env, e); return JNI_FALSE; }
    };
    match engine.update_params_json(&params) {
        Ok(()) => JNI_TRUE,
        Err(e) => {
            engine.last_error = Some(e.clone());
            throw(&mut env, e);
            JNI_FALSE
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_latentcam_spectralfilm_SpectralEngine_nativeSoftUpdateParamsJson(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    patch_json: JString,
) -> jboolean {
    init_logger();
    let Some(engine) = (unsafe { engine_mut(handle) }) else {
        throw(&mut env, "Invalid native engine handle");
        return JNI_FALSE;
    };
    let patch = match get_string(&mut env, &patch_json) {
        Ok(s) => s,
        Err(e) => { throw(&mut env, e); return JNI_FALSE; }
    };
    match engine.soft_update_params_json(&patch) {
        Ok(()) => JNI_TRUE,
        Err(e) => {
            engine.last_error = Some(e.clone());
            throw(&mut env, e);
            JNI_FALSE
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_latentcam_spectralfilm_SpectralEngine_nativeSoftUpdateExposurePrintFilters(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    exposure_compensation_ev: jdouble,
    auto_exposure_mode: jint,
    print_exposure: jdouble,
    y_filter_shift: jdouble,
    m_filter_shift: jdouble,
    c_filter_neutral: jdouble,
    y_filter_neutral: jdouble,
    m_filter_neutral: jdouble,
) -> jboolean {
    init_logger();
    let Some(engine) = (unsafe { engine_mut(handle) }) else {
        throw(&mut env, "Invalid native engine handle");
        return JNI_FALSE;
    };
    engine.soft_update_exposure_print_filters(
        exposure_compensation_ev,
        auto_exposure_mode,
        print_exposure,
        y_filter_shift,
        m_filter_shift,
        c_filter_neutral,
        y_filter_neutral,
        m_filter_neutral,
    );
    JNI_TRUE
}

#[no_mangle]
pub extern "system" fn Java_com_latentcam_spectralfilm_SpectralEngine_nativeGetParamsJson(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jstring {
    let Some(engine) = (unsafe { engine_mut(handle) }) else {
        throw(&mut env, "Invalid native engine handle");
        return ptr::null_mut();
    };
    match engine.params.to_json() {
        Ok(s) => match env.new_string(s) {
            Ok(s) => s.into_raw(),
            Err(e) => { throw(&mut env, e.to_string()); ptr::null_mut() }
        },
        Err(e) => {
            let msg = e.to_string();
            engine.last_error = Some(msg.clone());
            throw(&mut env, msg);
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_latentcam_spectralfilm_SpectralEngine_nativeGetLastWidth(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jint {
    unsafe { engine_mut(handle).map(|e| e.last_width as jint).unwrap_or(0) }
}

#[no_mangle]
pub extern "system" fn Java_com_latentcam_spectralfilm_SpectralEngine_nativeGetLastHeight(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jint {
    unsafe { engine_mut(handle).map(|e| e.last_height as jint).unwrap_or(0) }
}

#[no_mangle]
pub extern "system" fn Java_com_latentcam_spectralfilm_SpectralEngine_nativeGetLastChannels(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jint {
    unsafe { engine_mut(handle).map(|e| e.last_channels as jint).unwrap_or(0) }
}

#[no_mangle]
pub extern "system" fn Java_com_latentcam_spectralfilm_SpectralEngine_nativeGetTimings(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jstring {
    let s = unsafe { engine_mut(handle).map(|e| e.pipeline.format_timings()).unwrap_or_else(|| "Invalid handle".into()) };
    match env.new_string(s) {
        Ok(s) => s.into_raw(),
        Err(_) => env.new_string("").expect("empty Java string").into_raw(),
    }
}

// ---------------------------------------------------------------------------
// C ABI. These functions are useful for tests, Unity/Unreal integrations, or
// custom Android wrappers that do not use the Java package above.
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn spectralfilm_create_from_params_json(
    params_json: *const c_char,
) -> *mut Engine {
    init_logger();
    if params_json.is_null() { return ptr::null_mut(); }
    let params = unsafe { CStr::from_ptr(params_json) }.to_string_lossy().to_string();
    match Engine::from_params_json(&params) {
        Ok(e) => Box::into_raw(Box::new(e)),
        Err(e) => {
            error!("spectralfilm_create_from_params_json failed: {e}");
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn spectralfilm_create_from_json(
    film_json: *const c_char,
    print_json: *const c_char,
) -> *mut Engine {
    init_logger();
    if film_json.is_null() || print_json.is_null() { return ptr::null_mut(); }
    let film = unsafe { CStr::from_ptr(film_json) }.to_string_lossy().to_string();
    let print = unsafe { CStr::from_ptr(print_json) }.to_string_lossy().to_string();
    match Engine::new(&film, &print) {
        Ok(e) => Box::into_raw(Box::new(e)),
        Err(e) => {
            error!("spectralfilm_create_from_json failed: {e}");
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn spectralfilm_update_params_json(
    engine: *mut Engine,
    params_json: *const c_char,
) -> bool {
    if engine.is_null() || params_json.is_null() { return false; }
    let engine = unsafe { &mut *engine };
    let params = unsafe { CStr::from_ptr(params_json) }.to_string_lossy().to_string();
    match engine.update_params_json(&params) {
        Ok(()) => true,
        Err(e) => {
            engine.last_error = Some(e);
            false
        }
    }
}

#[no_mangle]
pub extern "C" fn spectralfilm_soft_update_params_json(
    engine: *mut Engine,
    patch_json: *const c_char,
) -> bool {
    if engine.is_null() || patch_json.is_null() { return false; }
    let engine = unsafe { &mut *engine };
    let patch = unsafe { CStr::from_ptr(patch_json) }.to_string_lossy().to_string();
    match engine.soft_update_params_json(&patch) {
        Ok(()) => true,
        Err(e) => {
            engine.last_error = Some(e);
            false
        }
    }
}

#[no_mangle]
pub extern "C" fn spectralfilm_soft_update_exposure_print_filters(
    engine: *mut Engine,
    exposure_compensation_ev: f64,
    auto_exposure_mode: i32,
    print_exposure: f64,
    y_filter_shift: f64,
    m_filter_shift: f64,
    c_filter_neutral: f64,
    y_filter_neutral: f64,
    m_filter_neutral: f64,
) -> bool {
    if engine.is_null() { return false; }
    let engine = unsafe { &mut *engine };
    engine.soft_update_exposure_print_filters(
        exposure_compensation_ev,
        auto_exposure_mode,
        print_exposure,
        y_filter_shift,
        m_filter_shift,
        c_filter_neutral,
        y_filter_neutral,
        m_filter_neutral,
    );
    true
}

#[no_mangle]
pub extern "C" fn spectralfilm_runtime_params_json(engine: *const Engine) -> *mut c_char {
    if engine.is_null() { return ptr::null_mut(); }
    let params = unsafe { &(*engine).params };
    match params.to_json().ok().and_then(|s| CString::new(s).ok()) {
        Some(s) => s.into_raw(),
        None => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn spectralfilm_destroy(engine: *mut Engine) {
    if !engine.is_null() {
        unsafe { drop(Box::from_raw(engine)); }
    }
}

#[no_mangle]
pub extern "C" fn spectralfilm_process_f32(
    engine: *mut Engine,
    input: *const f32,
    width: usize,
    height: usize,
    output: *mut f32,
    output_capacity: usize,
) -> usize {
    if engine.is_null() || input.is_null() || output.is_null() || width == 0 || height == 0 {
        return 0;
    }
    let n_in = width * height * 3;
    let input = unsafe { std::slice::from_raw_parts(input, n_in) };
    let engine = unsafe { &mut *engine };
    match engine.process_f32(input, width, height) {
        Ok(result) => {
            let n = result.len().min(output_capacity);
            unsafe { std::slice::from_raw_parts_mut(output, n).copy_from_slice(&result[..n]); }
            n
        }
        Err(e) => {
            engine.last_error = Some(e);
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn spectralfilm_last_width(engine: *const Engine) -> usize {
    if engine.is_null() { 0 } else { unsafe { (*engine).last_width } }
}

#[no_mangle]
pub extern "C" fn spectralfilm_last_height(engine: *const Engine) -> usize {
    if engine.is_null() { 0 } else { unsafe { (*engine).last_height } }
}

#[no_mangle]
pub extern "C" fn spectralfilm_last_error(engine: *const Engine) -> *mut c_char {
    if engine.is_null() { return ptr::null_mut(); }
    let msg = unsafe { (*engine).last_error.clone().unwrap_or_default() };
    CString::new(msg).map(CString::into_raw).unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn spectralfilm_free_string(s: *mut c_char) {
    if !s.is_null() { unsafe { drop(CString::from_raw(s)); } }
}
