
console.log(findWindow("神器传说"))
console.log("窗口宽高：("+windowWidth()+", "+windowHeight()+")");
activeWindow();

var kill_all = false;

function logStartFlag(name ) {
    console.log("查找【"+name+"】标识");
}
function logFindPos(name) {
    console.log("【" +name + "】标志位置（" + findX() + ", " + findY() + ")");
}

function logFindPosError(name) {
    console.log("【" +name + "】标志位置查找失败");
}

function logFindPosAndClickInfo(name) {
    console.log("【" +name + "】标志位置（" + findX() + ", " + findY() + "), 并点击");
}

function hasJxTzFlag() {
    var findFlagName = "继续挑战";
    logStartFlag(findFlagName);
    var ncc = findTemplate("assets/zhenfa_jxtz.png", 427, 1396, 390, 235);
    if (ncc>0.8) {
        var click_x = findX();
        var click_y = findY();
        if (!kill_all) {
            if (hasEndFlag()){
                console.log("最后一局，主动退出！。。。。。");
                exit(0);
            }
        }
        click(click_x+(240/2), click_y+(86/2));
        logFindPosAndClickInfo(findFlagName);
        return true;
    }
    logFindPosError(findFlagName);
    return false;
}

function hasShiBaiFlag() {
    var findFlagName = "失败";
    logStartFlag(findFlagName);
    var ncc = findTemplate("assets/tzsb.png", 227, 474, 508, 273);
    if (ncc>0.8) {
        var row_size = windowHeight()/3
        var top = row_size*2;
        var ncc = findTemplate("assets/click_anywhere_close.png", 0, top, windowWidth(), row_size);
        if (ncc>0.8) {
            click(findX()+400, findY()+200);
        }
        return true;
    }
    logFindPosError(findFlagName);
    return false;
}

function hasShengLiFlag() {
    var findFlagName = "胜利";
    logStartFlag(findFlagName);
    var ncc = findTemplate("assets/tzcg.png", 227, 474, 508, 273);
    if (ncc>0.8) {
        var row_size = windowHeight()/3
        var top = row_size*2;
        var ncc = findTemplate("assets/click_anywhere_close.png", 0, top, windowWidth(), row_size);
        if (ncc>0.8) {
            click(findX()+400, findY()+200);
        }
        return true;
    }
    logFindPosError(findFlagName);
    return false;
}

function hasNanDuFlag() {
    var findFlagName = "阵法难度提示";
    logStartFlag(findFlagName);
    var ncc = findTemplate("assets/zhenfa_nandu_tishi.png", 0, 350, windowWidth(),  200);
    if (ncc>0.8) {
        var top = windowHeight()/2;
        var ncc = findTemplate("assets/zhenfa_zise_jiangli_da.png", 0, top, windowWidth(), top);
        if (ncc>0.8) {
            click(findX(), findY());
        }else{
            var ncc = findTemplate("assets/zhenfa_zise_jiangli.png", 0, top, windowWidth(), top);
            if (ncc>0.8) {
                click(findX(), findY());
            }
        }
        var ncc = findTemplate("assets/zhenfa_nandu_xuanze.png", 0, top, windowWidth(), top);
        if (ncc>0.8) {
            click();
        }
        return true;
    }
    logFindPosError(findFlagName);
    return false;
}

function hasEndFlag() {
    var findFlagName = "最后一局";
    logStartFlag(findFlagName);
    var ncc = findTemplate("assets/zhenfa_jiangli_box.png", 699, 1041, 110, 110);
    if (ncc>0.8) {
        return false;
    }
    return true;
}


var next_click_count =0;
var is_pause = true;
var sleep_ms = 1000;

function isPause(){
    if (isKeyDown("LControl", "S")){
        console.log("[LControl+S] 按下！。。。。。");
        is_pause = false;
    }
    if (isKeyDown("LControl", "D")){
        console.log("[LControl+D] 按下！。。。。。");
        is_pause = false;
        kill_all = true;
    }
    if (isKeyDown("LControl", "A")){
        console.log("[LControl+A] 按下！。。。。。");
        is_pause = true;
    }
    if (is_pause){
        console.log("已暂停！。。。。。");
        sleep(sleep_ms);
        return true;
    }
    sleep(sleep_ms)
    return false;
}

while(1) {
    if (isPause()){
        continue;
    }
    if (hasJxTzFlag()){
        next_click_count++;
        if (next_click_count>5){
            console.log("挑战次数被限制，主动退出！。。。。。");
            break;
        }
    }
    if (isPause()){
        continue;
    }
    if (hasShengLiFlag()){
        next_click_count = 0;
    }
    if (isPause()){
        continue;
    }
    if (hasShiBaiFlag()){
        next_click_count = 0;
    }
    if (isPause()){
        continue;
    }
    if (hasNanDuFlag()){
        next_click_count = 0;
    }
}