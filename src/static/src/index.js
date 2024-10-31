// Stub enough of process to make browserify's util happy...
process = { env: {} }

let chai = require('chai')
format = require('util').format;
assert = chai.assert;
expect = chai.expect;
should = chai.should;
