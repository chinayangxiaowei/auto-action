# auto-action
Writing automated windows using javascript


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

## Log Printing
console.log(x:any,[...])