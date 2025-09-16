// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

import "forge-std/Test.sol";
import "../src/VaporPay.sol";
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract TestToken is ERC20 {
    constructor() ERC20("TestToken", "TTK") {
        _mint(msg.sender, 1000 ether);
    }
}

contract VaporPayERC20Test is Test {
    VaporPay vapor;
    TestToken token;
    address fallbackCollector = address(0x1234);
    address sender = address(this);
    address redeemer = address(0xBEEF);

    function setUp() public {
        vapor = new VaporPay(fallbackCollector);
        token = new TestToken();
        token.approve(address(vapor), type(uint256).max);
    }

    function testDepositAndRedeemERC20() public {
        bytes32 secret = keccak256("s1");
        bytes32 salt   = keccak256("salt1");
        bytes32 commitment = keccak256(abi.encodePacked(secret, salt));

        vapor.depositERC20(commitment, address(token), 10 ether, block.timestamp + 1 days, true);

        uint256 beforeBal = token.balanceOf(redeemer);
        vapor.redeem(secret, salt, redeemer);
        assertEq(token.balanceOf(redeemer), beforeBal + 10 ether);
    }

    function testRefundERC20() public {
        bytes32 secret = keccak256("s2");
        bytes32 salt   = keccak256("salt2");
        bytes32 commitment = keccak256(abi.encodePacked(secret, salt));

        vapor.depositERC20(commitment, address(token), 5 ether, block.timestamp + 1, true);

        vm.warp(block.timestamp + 2);

        uint256 beforeBal = token.balanceOf(fallbackCollector);
        vapor.refund(commitment, address(0));
        assertEq(token.balanceOf(fallbackCollector), beforeBal + 5 ether);
    }
}
