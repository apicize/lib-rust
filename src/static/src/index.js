// Stub enough of process to make browserify's util happy...
process = { env: {} }

const chai = require('chai');
const format = require('util').format;

const jpp = require('jsonpath-plus');
const xmldom = require('@xmldom/xmldom');

let testOffset = 0;
let names = [];
let inIt = false;
let results = [];
let logs = []

function fmtMinSec(value, subZero = null) {
    if (value === 0 && subZero) {
        return subZero
    }
    const m = Math.floor(value / 60000)
    value -= m * 60000
    const s = Math.floor(value / 1000)
    value -= s * 1000
    return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}${(0.1).toString()[1]}${value.toString().padEnd(3, '0')}`
}

function appendLog(type, message, ...optionalParams) {
    const timestamp = fmtMinSec(Date.now() - testOffset)
    logs.push(`${timestamp} [${type}] ${format(message, ...optionalParams)}`)
}

function clearLog() {
    logs = [];
}

function pushResult(result) {
    results.push({
        ...result,
        logs: logs.length > 0 ? logs : undefined
    });
}

/******************************************************************
 * Variables exposed to test runner
 ******************************************************************/

request = {};
response = {};
variables = {};

assert = chai.assert;
expect = chai.expect;
should = chai.should;

jsonpath = jpp.JSONPath;
xpath = require('xpath');
dom = xmldom.DOMParser;

console = {
    log: (msg, ...args) => appendLog('log', msg, ...args),
    info: (msg, ...args) => appendLog('info', msg, ...args),
    warn: (msg, ...args) => appendLog('warn', msg, ...args),
    error: (msg, ...args) => appendLog('error', msg, ...args),
    trace: (msg, ...args) => appendLog('trace', msg, ...args),
    debug: (msg, ...args) => appendLog('debug', msg, ...args),
};

describe = (name, run) => {
    names.push(name);
    try {
        run()
    } finally {
        names.pop();
    }
};

it = (behavior, run) => {
    try {
        if (inIt) {
            throw new Error('\"it\" cannot be contained in another \"it\" block')
        }
        if (names.length === 0) {
            throw new Error('\"it\" must be contained in a \"describe\" block');
        }
        inIt = true;
        run();
        pushResult({ testName: [...names, behavior], success: true });
        clearLog();
    } catch (e) {
        pushResult({ testName: [...names, behavior], success: false, error: e.message })
        clearLog()
    } finally {
        inIt = false
    }
};

runTestSuite = (request1, response1, variables1, testOffset1, testSuite) => {
    request = request1
    response = response1
    variables = variables1 ?? {}
    testOffset = testOffset1
    // console.log('variables', variables)
    names = []
    clearLog()
    results = []
    testSuite()
    return JSON.stringify({
        results,
        variables
    })
};
