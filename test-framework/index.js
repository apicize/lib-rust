// Stub enough of process to make browserify's util happy...
process = { env: {} }

const chai = require('chai');
const format = require('util').format;

const jpp = require('jsonpath-plus');
// const xmldom = require('@xmldom/xmldom');

let testOffset = 0;
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

/******************************************************************
 * Global variables exposed to test runner
 ******************************************************************/

request = {};
response = {};
variables = {};
scenario = {};
data = {};
$ = {};
outputVars = {};

assert = chai.assert;
expect = chai.expect;
should = chai.should;

jsonpath = jpp.JSONPath;
// xpath = require('xpath');
// dom = xmldom.DOMParser;

// Helper function to jsonpath-plus
function jpath(param) {
    if (typeof param === 'object') {
        return jpp.JSONPath({...param, json: this})
    } else if (typeof param === 'string') {
        const result = jpp.JSONPath({json: this, path: param})
        return (Array.isArray(result) && result.length === 1) 
            ? result[0] : result        
    } else {
        throw new Error('Argument for jp must be either a JSON path (string) or named parameters')
    }
}

Object.prototype.jp = jpath
Array.prototype.jp = jpath
String.prototype.jp = jpath
Number.prototype.jp = jpath

console = {
    log: (msg, ...args) => appendLog('log', msg, ...args),
    info: (msg, ...args) => appendLog('info', msg, ...args),
    warn: (msg, ...args) => appendLog('warn', msg, ...args),
    error: (msg, ...args) => appendLog('error', msg, ...args),
    trace: (msg, ...args) => appendLog('trace', msg, ...args),
    debug: (msg, ...args) => appendLog('debug', msg, ...args),
};

BodyType = {
    JSON: 'JSON',
    XML: 'XML',
    Text: 'Text',
    Form: 'Form',
    Binary: 'Binary'
}

let results = []
let current_results = results
let results_queue = [ results ]
let inIt = false;

function pushResult(result) {
    current_results.push({
        ...result,
        logs: logs.length > 0 ? logs : undefined
    });
    clearLog();
}

function updateTallies(entry) {
    if (entry.children) {
        entry.passed = true
        for (const child of entry.children) {
            updateTallies(child)
            entry.testCount += child.testCount
            entry.testFailCount += child.testFailCount
            entry.passed &&= child.passed
        }
    } else {
        entry.testCount = 1
        entry.testFailCount = entry.passed ? 0 : 1
    }
}


describe = (name, run) => {    
    let entry = {
        type: 'Scenario', 
        name,
        success: true,
        children: [],
        testCount: 0,
        testFailCount: 0,
    }
    
    current_results.push(entry)

    results_queue.push(current_results)
    current_results = entry.children

    try {
        run()
    } finally {
        updateTallies(entry)
        current_results = results_queue.pop()
    }
};

it = (name, run) => {
    try {
        if (inIt) {
            throw new Error('\"it\" cannot be contained in another \"it\" block')
        }
        inIt = true;
        run();
        pushResult({ type: 'Behavior', name, success: true, testCount: 1, testFailCount: 0 });
    } catch (e) {
        pushResult({ type: 'Behavior', name, success: false, testCount: 1, testFailCount: 0, error: e.message })
    } finally {
        inIt = false
    }
};

output = (name, value) => {
    switch (typeof value) {
        case 'function':
            throw new Error('Functions cannot be output')
        case 'symbol':
            throw new Error('Symbols cannot be output')
        case 'undefined':
            delete outputVars[name]
            break
        default:
            outputVars[name] = value
            break
    }
}

runTestSuite = (request1, response1, variables1, data1, output1, testOffset1, testSuite) => {
    request = request1
    response = response1
    scenario = variables1 ?? {}
    data = data1 ?? {}
    outputVars = output1 ?? {}
    
    $ = {...outputVars, ...scenario, ...data}
    variables = $ // retain variables for some level of backward compatibility

    testOffset = testOffset1
    clearLog()
    testSuite()
    return JSON.stringify({
        results,
        output: outputVars
    })
};
