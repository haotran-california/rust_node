let filename = "src/testing/temp_read_file.txt"

fs.readFile(filename, (err, data) =>{
    console.log("File Data: ")
    console.log(data)
})