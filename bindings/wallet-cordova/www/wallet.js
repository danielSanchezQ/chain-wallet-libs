/*
    TODO: Add license?
 */

var exec = require('cordova/exec');

const NATIVE_CLASS_NAME = 'WalletPlugin';

const WALLET_RESTORE_ACTION_TAG = 'WALLET_RESTORE';
const WALLET_RETRIEVE_FUNDS_ACTION_TAG = 'WALLET_RETRIEVE_FUNDS';
const WALLET_TOTAL_FUNDS_ACTION_TAG = 'WALLET_TOTAL_FUNDS';
const WALLET_ID_TAG = 'WALLET_ID';
const WALLET_DELETE_ACTION_TAG = 'WALLET_DELETE';
const SETTINGS_DELETE_ACTION_TAG = 'SETTINGS_DELETE';

function _arrayBufferToBase64 (buffer) {
    var binary = '';
    var bytes = new Uint8Array(buffer);
    var len = bytes.byteLength;
    for (var i = 0; i < len; i++) {
        binary += String.fromCharCode(bytes[i]);
    }
    return window.btoa(binary);
}

/**
 * THOUGHTS/TODO
 * add a more idiomatic abstraction on top of these primitive functions and expose that, something more similar to what wasm-bindgen does
 * I'm still not sure what javascript features can we use here (ES6, can we bring dependencies?, promises?)
*/

var plugin = {
    /**
     * @param {string} mnemonics description (TODO)
     * @param {function} successCallback description (TODO)
     * @param {function} errorCallback description (TODO)
     */
    walletRestore: function (mnemonics, successCallback, errorCallback) {
        console.log(mnemonics);
        exec(successCallback, errorCallback, NATIVE_CLASS_NAME, WALLET_RESTORE_ACTION_TAG, [mnemonics]);
    },

    /**
     * @param block0 description (TODO)
     * @param {function} successCallback description (TODO)
     * @param {function} errorCallback description (TODO)
     */
    walletRetrieveFunds: function (ptr, block0, successCallback, errorCallback) {
        const base64 = _arrayBufferToBase64(block0);
        exec(successCallback, errorCallback, NATIVE_CLASS_NAME, WALLET_RETRIEVE_FUNDS_ACTION_TAG, [ptr, base64]);
    },

    /**
     * @param {string} native_id description (TODO)
     * @param {string} successCallback description (TODO)
     * @param {string} errorCallback description (TODO)
     */
    walletTotalFunds: function (ptr, successCallback, errorCallback) {
        exec(successCallback, errorCallback, NATIVE_CLASS_NAME, WALLET_TOTAL_FUNDS_ACTION_TAG, [ptr]);
    },

    /**
     * get the wallet id

     * This ID is the identifier to use against the blockchain/explorer to retrieve
     * the state of the wallet (counter, total value etc...)
     *
     * # Safety
     *
     * This function dereference raw pointers (wallet). Even though
     * the function checks if the pointers are null. Mind not to put random values
     * in or you may see unexpected behaviors
     *
     * @param {number} ptr a pointer to a Wallet object obtained with WalletRestore
     * @param {function} successCallback the return value is an ArrayBuffer, which has the binary representation of the account id.
     * @param {function} errorCallback this function may fail if the wallet pointer is null
     */
    walletId: function (ptr, successCallback, errorCallback) {
        exec(successCallback, errorCallback, NATIVE_CLASS_NAME, WALLET_ID_TAG, [ptr]);
    },

    /**
     * @param {string} native_id description (TODO)
     * @param {string} successCallback description (TODO)
     * @param {string} errorCallback description (TODO)
     */
    walletDelete: function (ptr, successCallback, errorCallback) {
        exec(successCallback, errorCallback, NATIVE_CLASS_NAME, WALLET_DELETE_ACTION_TAG, [ptr]);
    },

    /**
     * @param {string} native_id description (TODO)
     * @param {string} successCallback description (TODO)
     * @param {string} errorCallback description (TODO)
     */
    settingsDelete: function (ptr, successCallback, errorCallback) {
        exec(successCallback, errorCallback, NATIVE_CLASS_NAME, SETTINGS_DELETE_ACTION_TAG, [ptr]);
    }
};

module.exports = plugin;
