
function async_read_file(path, encoding="", flags="r", callback=None){
    console.log("you have read the function")
    nodeFs.readFile(path)
}