// Define a console object
var console = {
    print: function(input) {
        log(input);  // Use the Rust binding 'log' function
    }
};

// Return console object 
console; 