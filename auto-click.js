
console.log(findWindow("神器传说"))
console.log("窗口宽高：("+windowWidth()+", "+windowHeight()+")");
activeWindow();

var is_pause = true;
var sleep_ms = 2000;

function isPause(){
    if (isKeyDown("LControl", "S")){
        console.log("[LControl+S] 按下！。。。。。");
        is_pause = false;
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
    click(232*2, 750*2);
}