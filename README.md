# auto-action
Automated window control in JavaScript, using a template image search for positioning.  
Tested only on macOS and under the RustRover editor, running under RustRover's built-in console.  
Screen access and keyboard control may need to be enabled to run.

# API Interface

## Finds a window that will be the target of a later operation
function findWindow(title:string):boolean

## Get the width and height of the window
function windowWidth():number  
function windowHeight():number  

## Activate target window
function activeWindow():boolean

## delay function
sleep(n:number)

## Find out if the template image is in the window，Return match rate
findTemplate("assets/xxx.png", x:number, y:number, w:number, h:number):number  
You can use findX(), findY() to get the found coordinates  

## Click on the target location
click([x:number, y:number]);   
click(); //x=findX(), y=findY()

## Get global key status
isKeyDown(keyName:string, [...]):boolean
[View KeyName](https://github.com/ostrosco/device_query/blob/master/src/keymap.rs)

## Log Printing
console.log(x:any,[...])
