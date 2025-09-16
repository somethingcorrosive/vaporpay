// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import "forge-std/Test.sol";
import "../src/VaporPay.sol";

contract VaporPayETHTest is Test {
    VaporPay vapor;
    address fallbackCollector = address(0x1234);

    function setUp() public {
        vapor = new VaporPay(fallbackCollector);
    }

    function testDepositAndRedeemETH() public {
        bytes32 secret = keccak256("secret");
        bytes32 salt   = keccak256("salt");
        bytes32 commitment = keccak256(abi.encodePacked(secret, salt));

        // Deposit
        vapor.depositETH{value: 1 ether}(commitment, block.timestamp + 1 days, true);

        // Redeem
        address redeemer = address(0xABCD);
        uint256 balBefore = redeemer.balance;
        vapor.redeem(secret, salt, redeemer);
        assertEq(redeemer.balance, balBefore + 1 ether);
    }

    function testRefundETH() public {
        bytes32 secret = keccak256("secret2");
        bytes32 salt   = keccak256("salt2");
        bytes32 commitment = keccak256(abi.encodePacked(secret, salt));

        // Deposit
        vapor.depositETH{value: 1 ether}(commitment, block.timestamp + 1, true);

        // Warp forward so expired
        vm.warp(block.timestamp + 2);

        // Refund to fallback
        uint256 balBefore = fallbackCollector.balance;
        vapor.refund(commitment, address(0));
        assertEq(fallbackCollector.balance, balBefore + 1 ether);
    }
}
