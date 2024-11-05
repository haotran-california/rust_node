get('http://localhost:8000/authors', (res) => {
    res.on('data', (chunk) => {
        console.log("Data Chunk: ")
        console.log(chunk)
    })
})