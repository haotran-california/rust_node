// Define a console object
var console = {
    print: function(input) {
        log(input);  // Use the Rust binding 'log' function
        return input;
    },
    name: "console name"
};

// Return console object 
console; 