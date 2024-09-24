// Define the Coin class
class Coin {
    constructor(value, currency, face_up, year) {
      this.value = value;         // Value of the coin
      this.currency = currency;   // Currency type (e.g., dollar)
      this.face_up = face_up;     // Face up side (e.g., heads or tails)
      this.year = year;           // Year of the coin's minting
    }
  
    // You can add more methods if needed, like flipping the coin
    flip() {
      this.face_up = Math.random() > 0.5 ? 'heads' : 'tails';
    }
  }
  
  // Initialize a few Coin instances
  export default quarter = new Coin(0.25, "dollar", "heads", 1987);

  