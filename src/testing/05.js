let filename = "src/testing/temp_write_file.txt"
let content = "New file contents"

writeFile(filename, content, (err, data) =>{
    console.log("File Data: ")
    console.log(data)
})