use crate::string::String;
use std::time::Instant;
use std::{env, process};
use std::fs;
use std::io::Write;
use std::string;
use cocoa::base::{id, nil};
use xcap::Window;

extern crate objc;
use objc::runtime::{Class, Object};
use objc::{class, msg_send, sel, sel_impl};
use image::{open, DynamicImage, GenericImage, ImageBuffer, Luma, Rgba};
use image::GenericImageView;
use std::{thread, time};
use std::error::Error;
use std::ptr::null_mut;
use rayon::prelude::*; // 引入 rayon 的并行迭代器
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use once_cell::sync::Lazy;

use boa::{Context, JsResult, JsValue, object::ObjectInitializer, property::Attribute};
use boa::JsValue::Null;
use log::{info, debug, warn, error};

use enigo::{
    Button,
    Direction::{Click, Press, Release},
    Enigo, Mouse, Settings,Key,
    {Axis::Horizontal, Axis::Vertical},
    {Coordinate::Abs, Coordinate::Rel},
};
use env_logger::{Builder, Env};
use image::imageops::flip_horizontal;

use device_query::{DeviceQuery, DeviceState, Keycode};

fn get_backing_scale_factor() -> f32 {
    unsafe {
        // 获取 NSScreen 类
        let ns_screen_class: *const Class = Class::get("NSScreen").expect("Failed to get NSScreen class");

        // 获取主屏幕对象
        let main_screen: *mut Object = msg_send![ns_screen_class, mainScreen];
        if main_screen.is_null() {
            panic!("Failed to get main screen");
        }

        // 获取 backingScaleFactor
        let scale_factor: f64 = msg_send![main_screen, backingScaleFactor];
        scale_factor as f32
    }
}


/// 计算两个图像区域之间的归一化互相关 (Normalized Cross-Correlation, NCC)
fn normalized_cross_correlation(
    img_region: &[u8],
    template: &[u8],
    template_size: usize,
    template_mean: f32,
    template_stddev: f32,
) -> f32 {

    // 计算图像区域的均值和标准差
    let img_mean = img_region.iter().map(|&x| x as u32).sum::<u32>() as f32 / template_size as f32;
    let img_variance = img_region
        .iter()
        .map(|&x| (x as f32 - img_mean).powi(2))
        .sum::<f32>() / template_size as f32;
    let img_stddev = img_variance.sqrt();

    // 如果标准差为0，避免除以0
    if img_stddev == 0.0 || template_stddev == 0.0 {
        return 0.0;
    }

    // 计算归一化互相关
    let mut ncc_sum = 0.0;
    for i in 0..template_size {
        ncc_sum += ((img_region[i] as f32 - img_mean) * (template[i] as f32 - template_mean)) / (img_stddev * template_stddev);
    }
    ncc_sum / template_size as f32
}

/// 模板匹配函数，返回最佳匹配位置的坐标
fn match_template(img: &DynamicImage, template: &DynamicImage, best_ncc: f32) -> Option<(u32, u32, f32)> {
    // 将图像转换为灰度图
    let img_gray = img.to_luma8();
    let template_gray = template.to_luma8();

    // 获取图像尺寸
    let (img_width, img_height) = img_gray.dimensions();
    let (template_width, template_height) = template_gray.dimensions();

    // 确保模板尺寸不大于图像尺寸
    if template_width > img_width || template_height > img_height {
        println!("模板尺寸不能大于图像尺寸！");
        return None;
    }

    // 将模板图像数据转换为向量
    let template_data: Vec<u8> = template_gray.into_raw();
    let template_size = (template_width * template_height) as usize;

    // 计算模板的均值和标准差
    let template_mean = template_data.iter().map(|&x| x as u32).sum::<u32>() as f32 / template_size as f32;
    let template_variance = template_data
        .iter()
        .map(|&x| (x as f32 - template_mean).powi(2))
        .sum::<f32>() / template_size as f32;
    let template_stddev = template_variance.sqrt();

    // 使用 Arc 和 Mutex 来共享和同步最大相关系数和最佳匹配位置
    let max_ncc = Arc::new(Mutex::new(f32::NEG_INFINITY));
    let best_match_pos = Arc::new(Mutex::new((0, 0, 0.0)));

    // 提前退出标志
    let stop_flag = Arc::new(AtomicBool::new(false));

    // 使用 rayon 的 par_iter 并行化外层的 y 循环
    (0..(img_height - template_height + 1)).into_par_iter().for_each(|y| {
        if stop_flag.load(Ordering::Relaxed) {
            return;
        }
        for x in 0..(img_width - template_width + 1) {
            // 使用 sub_image 创建一个子图像视图
            let img_sub_image = img_gray.view(x, y, template_width, template_height);

            // 将子图像视图转换为新的 ImageBuffer
            let img_region: ImageBuffer<Luma<u8>, Vec<u8>> = img_sub_image.to_image();

            // 提取当前窗口的图像区域数据
            let img_region_data: Vec<u8> = img_region.into_raw();

            // 计算归一化互相关
            let ncc = normalized_cross_correlation(&img_region_data, &template_data, template_size, template_mean, template_stddev);

            // 更新最大相关系数和最佳匹配位置
            let mut max_ncc_lock = max_ncc.lock().unwrap();
            let mut best_match_pos_lock = best_match_pos.lock().unwrap();
            if ncc > *max_ncc_lock {
                *max_ncc_lock = ncc;
                *best_match_pos_lock = (x, y, ncc);
                // 如果找到非常高的 NCC 值，设置提前退出标志
                if ncc > best_ncc {
                    stop_flag.store(true, Ordering::Relaxed);
                }
            }
        }
    });

    // 返回最佳匹配位置
    let result = Some(*best_match_pos.lock().unwrap()); result
}

// 定义一个用于处理 console.log 的 Rust 函数
fn js_console_log(_this: &JsValue, args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    // 将所有参数转换为字符串并连接起来
    let messages: String = args.iter().map(|arg| arg.display().to_string()).collect::<Vec<_>>().join(", ");
    // 使用 info! 宏记录日志信息
    info!("{}", messages);
    Ok(JsValue::undefined())
}

// 类似地，可以定义其他 console 方法，如 debug, warn, error
fn js_console_debug(_this: &JsValue, args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    let messages: String = args.iter().map(|arg| arg.display().to_string()).collect::<Vec<_>>().join(", ");
    debug!("{}", messages);
    Ok(JsValue::undefined())
}

fn js_console_warn(_this: &JsValue, args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    let messages: String = args.iter().map(|arg| arg.display().to_string()).collect::<Vec<_>>().join(", ");
    warn!("{}", messages);
    Ok(JsValue::undefined())
}

fn js_console_error(_this: &JsValue, args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    let messages: String = args.iter().map(|arg| arg.display().to_string()).collect::<Vec<_>>().join(", ");
    error!("{}", messages);
    Ok(JsValue::undefined())
}


static mut FIND_WINDOW_TITLE: String = String::new();
static mut FIND_TEMPLATE_X: u32 = 0;
static mut FIND_TEMPLATE_Y: u32 = 0;
static mut FIND_TEMPLATE_NCC: f32 = 0.0;

static mut LAST_WINDOW_WIDTH: u32 = 0;
static mut LAST_WINDOW_HEIGHT: u32 = 0;

static mut LAST_TEMPLATE_WIDTH: u32 = 0;
static mut LAST_TEMPLATE_HEIGHT: u32 = 0;


fn find_window(title:String) -> bool {
    let windows = Window::all().unwrap();
    let mut result = false;
    for window in windows {
        if window.title().to_string() == title {
            unsafe {
                FIND_WINDOW_TITLE = window.title().to_string();
                LAST_WINDOW_WIDTH = window.width();
                LAST_WINDOW_HEIGHT = window.height();
            }
            result = true;
            break;
        }
    }
    result
}

// 将 Rust 函数包装为可以在 JavaScript 中调用的形式
fn js_find_window(_this: &JsValue, args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    // 获取第一个参数（假设它是数字）
    let title = if let Some(arg) = args.get(0) {
        arg.as_string().map(|n| n)
    } else {
        None
    };

    // 如果没有提供有效的参数，则返回 undefined
    let result = match title  {
        Some(n) => find_window(n.to_string()).into(),
        None => JsValue::Boolean(false),
    };

    Ok(result)
}

fn active_window(pid: u32) -> bool {
    let app = Class::get("NSRunningApplication").unwrap();
    let running_app: id  = unsafe { msg_send![app, runningApplicationWithProcessIdentifier: pid] };
    if running_app != nil {
        let _: () = unsafe { msg_send![running_app, activateWithOptions: 0] };
        return true;
    }
    false
}


fn js_active_window(_this: &JsValue, _args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    let windows = Window::all().unwrap();
    for window in windows {
        if unsafe { window.title().to_string() == FIND_WINDOW_TITLE} {
            unsafe {
                LAST_WINDOW_WIDTH = window.width();
                LAST_WINDOW_HEIGHT = window.height();
            }
            // 激活应用程序
            return Ok(JsValue::Boolean(active_window(window.pid())));
        }
    }
    Ok(JsValue::Boolean(false))
}

/// 调整图像对比度
fn adjust_contrast(image: &DynamicImage, factor: f32) -> DynamicImage {
    let (width, height) = image.dimensions();
    let mut out_buffer = ImageBuffer::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let pixel = image.get_pixel(x, y);
            let new_pixel = apply_contrast_to_pixel(pixel, factor);
            out_buffer.put_pixel(x, y, new_pixel);
        }
    }

    DynamicImage::ImageRgba8(out_buffer)
}

/// 对单个像素应用对比度调整
fn apply_contrast_to_pixel(pixel: Rgba<u8>, factor: f32) -> Rgba<u8> {
    let f = |c: u8| -> u8 {
        let c_f32 = c as f32 / 255.0;
        let adjusted = (128.0 - (128.0 * factor) + (c_f32 * factor * 255.0)) as i32;
        let clamped = adjusted.clamp(0, 255);
        clamped as u8
    };

    Rgba([f(pixel[0]), f(pixel[1]), f(pixel[2]), pixel[3]])
}

fn find_template(template_file:String) -> f32 {
    let windows = Window::all().unwrap();
    let mut result = 0.0;
    for window in windows {
        if unsafe { window.title().to_string() == FIND_WINDOW_TITLE} {
            if window.is_minimized() {
                info!("窗口当前是最小化状态，自动激活窗口。");
                active_window(window.pid());
            }
            unsafe {
                LAST_WINDOW_WIDTH = window.width();
                LAST_WINDOW_HEIGHT = window.height();
            }
            if let Ok(template_image) = image::open(template_file.clone()) {
                info!("模版图片尺寸: ({}, {}), {}", template_image.width(), template_image.height(), template_file);
                let image = window.capture_image().unwrap();
                let dynamic_img: DynamicImage = DynamicImage::from(image);
                adjust_contrast(&dynamic_img, 1.5);
                //dynamic_img.save("dbd.png").unwrap();
                adjust_contrast(&template_image, 1.5);
                let start = Instant::now();
                let match_result = match_template(&dynamic_img, &template_image, 0.95);
                info!("匹配结果: {:?}, 耗时: {:?}", match_result, start.elapsed());
                if let Some((x, y, ncc)) = match_result {
                    info!("符合条件的坐标: ({}, {}), ncc: {}", x, y, ncc);
                    unsafe {
                        FIND_TEMPLATE_X = x;
                        FIND_TEMPLATE_Y = y;
                        FIND_TEMPLATE_NCC = ncc;
                        LAST_TEMPLATE_WIDTH = template_image.width();
                        LAST_TEMPLATE_HEIGHT = template_image.height();
                    }
                    result = ncc;
                } else {
                    info!("未找到符合条件的坐标");
                }
            } else {
                error!("打开模版文件失败，{:?}", template_file);
            }
            break;
        }
    }
    result
}

fn find_template_cut(template_file:String, offset_x: u32, offset_y: u32, width: u32, height: u32) -> f32 {
    let windows = Window::all().unwrap();
    let mut result = 0.0;
    for window in windows {
        if unsafe { window.title().to_string() == FIND_WINDOW_TITLE} {
            if window.is_minimized() {
                info!("窗口当前是最小化状态，自动激活窗口。");
                active_window(window.pid());
            }
            unsafe {
                LAST_WINDOW_WIDTH = window.width();
                LAST_WINDOW_HEIGHT = window.height();
            }
            if let Ok(template_image) = image::open(template_file.clone()) {
                info!("搜索区域: ({},{})-({},{})", offset_x, offset_y, offset_x+width, offset_y+height);
                info!("模版图片尺寸: ({}, {}), {}", template_image.width(), template_image.height(), template_file);
                if width<template_image.width() || height<template_image.height() {
                    error!("设置的搜索区域小于模版宽度，{}<{}, {}<{}", width, template_image.width(), height, template_image.height());
                    break;
                }
                let image = window.capture_image().unwrap();
                if (offset_x+width)>image.width() || (offset_y+height)>image.height() {
                    error!("设置的搜索区域超过窗口区域，{}+{}<{}, {}+{}<{}", offset_x, width, image.width(), offset_y, height, image.height());
                    break;
                }
                let sub_image = image.view(offset_x, offset_y, width, height);
                let dynamic_img: DynamicImage = DynamicImage::from(sub_image.to_image());
                dynamic_img.save(  format!("{}_cut_{}_{}.png",  template_file , offset_y, offset_y)).unwrap();
                adjust_contrast(&dynamic_img, 1.5);
                adjust_contrast(&template_image, 1.5);
                let start = Instant::now();
                let match_result = match_template(&dynamic_img, &template_image, 0.95);
                info!("匹配结果: {:?}, 耗时: {:?}", match_result, start.elapsed());
                if let Some((x, y, ncc)) = match_result {
                    info!("符合条件的坐标: ({}, {}), ncc: {}", offset_x+x, offset_y+y, ncc);
                    unsafe {
                        FIND_TEMPLATE_X = offset_x + x;
                        FIND_TEMPLATE_Y = offset_y + y;
                        FIND_TEMPLATE_NCC = ncc;
                        LAST_TEMPLATE_WIDTH = template_image.width();
                        LAST_TEMPLATE_HEIGHT = template_image.height();
                    }
                    result = ncc;
                } else {
                    info!("未找到符合条件的坐标");
                }
            } else {
                error!("打开模版文件失败，{:?}", template_file);
            }
            break;
        }
    }
    result
}


fn js_find_template(_this: &JsValue, _args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    // 获取第一个参数（假设它是数字）
    let png_file = if let Some(arg) = _args.get(0) {
        arg.as_string().map(|n| n)
    } else {
        None
    };

    let offset_x = if let Some(arg) = _args.get(1) {
        arg.as_number().map(|n| n).unwrap_or(0.0) as i32
    } else {
        0
    };

    let offset_y = if let Some(arg) = _args.get(2) {
        arg.as_number().map(|n| n).unwrap_or(0.0) as i32
    } else {
        0
    };

    let width = if let Some(arg) = _args.get(3) {
        arg.as_number().map(|n| n).unwrap_or(0.0) as i32
    } else {
        0
    };

    let height = if let Some(arg) = _args.get(4) {
        arg.as_number().map(|n| n).unwrap_or(0.0) as i32
    } else {
        0
    };

    // 如果没有提供有效的参数，则返回 undefined
    let result = match png_file  {
        Some(js_str) => {
            //let mut ncc:f32 = 0.0;
            if offset_x>=0 && offset_y>=0 && width>0 && height>0 {
                let ncc = find_template_cut(js_str.to_string(), offset_x as u32, offset_y as u32, width as u32, height as u32 ).into();
                JsValue::Rational(ncc)
            }else {
                let ncc = find_template(js_str.to_string()).into();
                JsValue::Rational(ncc)
            }
        },
        None => JsValue::Rational(0.0),
    };
    Ok(result)
}


fn js_click(_this: &JsValue, _args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    let offset_x = if let Some(arg) = _args.get(0) {
        arg.as_number().map(|n| n).unwrap_or(0.0) as i32
    } else {
        0
    };

    let offset_y = if let Some(arg) = _args.get(1) {
        arg.as_number().map(|n| n).unwrap_or(0.0) as i32
    } else {
        0
    };

    let screen_scale_factor = get_backing_scale_factor();

    let windows = Window::all().unwrap();
    for window in windows {
        unsafe {
            if window.title().to_string() == FIND_WINDOW_TITLE {
                LAST_WINDOW_WIDTH = window.width();
                LAST_WINDOW_HEIGHT = window.height();
                let mut enigo = Enigo::new(&Settings::default()).unwrap();
                if offset_x == 0 && offset_y == 0 {
                    let center_x = FIND_TEMPLATE_X+LAST_TEMPLATE_WIDTH/2;
                    let center_y = FIND_TEMPLATE_Y+LAST_TEMPLATE_HEIGHT/2;
                    info!("图片中心位置: {:?}, {:?}", center_x, center_y);
                    let click_x= center_x as f32 /screen_scale_factor;
                    let click_y= center_y as f32 /screen_scale_factor;
                    info!("点击中心位置: {:?}, {:?}", click_x, click_y);
                    enigo.move_mouse( window.x() + click_x as i32, window.y() + click_y as i32, Abs).unwrap();
                }else{
                    let click_x= offset_x as f32 /screen_scale_factor;
                    let click_y= offset_y as f32 /screen_scale_factor;
                    info!("点击位置: {:?}, {:?}", click_x, click_y);
                    enigo.move_mouse( window.x() + click_x as i32, window.y() + click_y as i32, Abs).unwrap();
                }
                thread::sleep(time::Duration::from_millis(500));
                enigo.button(Button::Left, Click).unwrap();
                return Ok(JsValue::Boolean(true));
            }
        }
    }
    Ok(JsValue::Boolean(false))
}

fn js_sleep(_this: &JsValue, _args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    let ms = if let Some(arg) = _args.get(0) {
        arg.as_number().map(|n| n).unwrap_or(0.0)
    } else {
        0.0
    };
    thread::sleep(time::Duration::from_millis(ms as u64));
    Ok(JsValue::undefined())
}

fn js_find_x(_this: &JsValue, _args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    unsafe { Ok(JsValue::Integer(FIND_TEMPLATE_X as i32)) }
}

fn js_find_y(_this: &JsValue, _args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    unsafe { Ok(JsValue::Integer(FIND_TEMPLATE_Y as i32)) }
}


fn js_window_width(_this: &JsValue, _args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    unsafe {
        let screen_scale_factor = get_backing_scale_factor();
        let mut width = LAST_WINDOW_WIDTH as f32;
        width = width * screen_scale_factor;
        Ok(JsValue::Integer(width as i32))
    }
}

fn js_window_height(_this: &JsValue, _args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    unsafe {
        let screen_scale_factor = get_backing_scale_factor();
        let mut width = LAST_WINDOW_HEIGHT as f32;
        width = width * screen_scale_factor;
        Ok(JsValue::Integer(width as i32))
    }
}

fn js_exit(_this: &JsValue, _args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    let exit_code = if let Some(arg) = _args.get(0) {
        arg.as_number().map(|n| n).unwrap_or(0.0)
    } else {
        0.0
    };
    info!("主动退出，退出码: {:?}", exit_code as i32);
    process::exit(exit_code as i32);
}

fn js_is_key_down(_this: &JsValue, args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    let mut result = true;
    let device_state = DeviceState::new();
    let mut keys = device_state.get_keys();
    for i in 0..10 {
        let key_name = if let Some(arg) = args.get(i) {
            arg.as_string()
        } else {
            None
        };

        if key_name!=None {
            let key_code =  key_name.expect("Key name is missing").parse().unwrap();
            if !keys.contains(&key_code) {
                result = false;
            }
        }
    }
    unsafe { Ok(JsValue::Boolean(result)) }
}

fn main() {

    // 获取命令行参数
    let args: Vec<String> = env::args().collect();

    // 初始化日志系统，设置日志级别为 info，并自定义日志格式
    Builder::from_env(Env::default().default_filter_or("info"))
        .format(|buf, record| {
            writeln!(
                buf,
                "[{}] [{} {}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                record.args()
            )
        })
        .init();


    // 创建一个新的 JavaScript 上下文
    let mut context = Context::default();

    // 创建一个自定义的 console 对象
    let console = ObjectInitializer::new(&mut context)
        .function(js_console_log, "log", 0) // 第三个参数是函数的参数长度
        .function(js_console_debug, "debug", 0)
        .function(js_console_warn, "warn", 0)
        .function(js_console_error, "error", 0)
        .build();

    // 将自定义的 console 对象添加到全局对象中，并设置属性特性
    context.register_global_property("console", console, Attribute::all());

    // 向全局对象添加一个名为 `findWindow` 的函数
    // 依据标题查找窗口
    let _ = context.register_global_function(
        "findWindow",
        1, // 标题
        js_find_window,
    );

    // 向全局对象添加一个名为 `activeWindow` 的函数
    // 激活窗口
    let _ = context.register_global_function(
        "activeWindow",
        0, // 无需参数
        js_active_window,
    );

    // 向全局对象添加一个名为 `findTemplate` 的函数
    // 查找图片所在坐标
    let _ = context.register_global_function(
        "findTemplate",
        1, // 文件名
        js_find_template,
    );

    // 向全局对象添加一个名为 `click` 的函数
    let _ = context.register_global_function(
        "click",
        0, // 无需参数
        js_click,
    );

    // 向全局对象添加一个名为 `sleep` 的函数
    let _ = context.register_global_function(
        "sleep",
        1, // 无需参数
        js_sleep,
    );

    // 向全局对象添加一个名为 `findX` 的函数
    // 用于返回上次查找到的图片坐标x位置
    let _ = context.register_global_function(
        "findX",
        0, // 无需参数
        js_find_x,
    );

    // 向全局对象添加一个名为 `findY` 的函数
    // 用于返回上次查找到的图片坐标y位置
    let _ = context.register_global_function(
        "findY",
        0, // 无需参数
        js_find_y,
    );

    // 向全局对象添加一个名为 `windowWidth` 的函数
    // 用于返回上次查找到的图片坐标x位置
    let _ = context.register_global_function(
        "windowWidth",
        0, // 无需参数
        js_window_width,
    );

    // 向全局对象添加一个名为 `windowHeight` 的函数
    // 用于返回上次查找到的图片坐标y位置
    let _ = context.register_global_function(
        "windowHeight",
        0, // 无需参数
        js_window_height,
    );

    // 向全局对象添加一个名为 `exit` 的函数
    // 用于直接退出进程
    let _ = context.register_global_function(
        "exit",
        0, // 无需参数
        js_exit,
    );

    // 向全局对象添加一个名为 `exit` 的函数
    // 用于直接退出进程
    let _ = context.register_global_function(
        "isKeyDown",
        0, // 无需参数
        js_is_key_down,
    );

    // 加载脚本
    let source = fs::read_to_string(args[1].to_string()).unwrap();

    // 执行代码
    let start = Instant::now();
    context.eval(source).unwrap();
    println!("运行耗时: {:?}", start.elapsed());

}