package main

import (
	"fmt"
	"io"
	"log"
	"net"
	"os"
)

var routers []chan<- []byte
var routing map[byte][]chan<- []byte
var subscriptions map[byte][]chan<- []byte

func bufferize(conn net.Conn, max int) ([]byte, error) {
	tmp := make([]byte, max)
	n, err := conn.Read(tmp)
	if err != nil {
		return nil, err
	} else {
		return tmp[:n], nil
	}
}

func channelContains(who chan<- []byte, where []chan<- []byte) int {
	for i, v := range where {
		if who == v {
			return i
		}
	}
	return -1
}

func frame(buf []byte) (byte, []byte, []byte) {
	if len(buf) > 0 {
		var s int
		s = int(buf[0])
		if len(buf) > s {
			rsize, rmsg, rbuf := buf[0], buf[1:s+1], buf[s+1:]
			return rsize, rmsg, rbuf
		}
	}
	return 0, nil, nil
}
func writer(conn net.Conn, comm <-chan []byte, label string, quit <-chan bool) {
	for {
		select {
		case packet := <-comm:
			log.Printf("[%v] ROUTED %v", label, packet)
			conn.Write([]byte{byte(len(packet))})
			conn.Write(packet)
		case <-quit:
			return
		}
	}
}

func publisher(conn net.Conn) {
	label := fmt.Sprintf("%v / PUBLISHER", conn.RemoteAddr())
	buf := make([]byte, 0, 1024)

	for {
		read, err := bufferize(conn, 8)
		if err != nil {
			log.Printf("[%v] ERROR %v", label, err)
			return
		}
		buf = append(buf, read...)
		for {
			size, msg, rest := frame(buf)
			if size == 0 {
				break
			} else {
				switch msg[0] {
				case 0x03:
					for _, ch := range subscriptions[msg[1]] {
						ch <- msg
					}
					for _, ch := range routing[msg[1]] {
						ch <- msg
					}
					log.Printf("[%v] PUBLISHED %v", label, msg)

				default:
					log.Printf("[%v] INVALID MESSAGE %v", label, msg)
				}
				buf = rest
			}
		}
	}
	log.Printf("[%v] CLOSED")
}

func router(conn net.Conn) {
	label := fmt.Sprintf("%v / ROUTER", conn.RemoteAddr())
	buf := make([]byte, 0, 1024)
	comm := make(chan []byte)
	quit := make(chan bool)
	defer conn.Close()

	// WRITE
	go writer(conn, comm, label, quit)

	routers = append(routers, comm)

	for {
		read, err := bufferize(conn, 8)
		if err != nil {
			log.Printf("[%v] ERROR %v", label, err)
			return
		}
		buf = append(buf, read...)
		for {
			size, msg, rest := frame(buf)
			if size == 0 {
				break
			} else {
				switch msg[0] {
				case 0x01:
					if channelContains(comm, routing[msg[1]]) == -1 {
						routing[msg[1]] = append(routing[msg[1]], comm)
						for _, ch := range routers {
							if ch != comm {
								ch <- msg
							}
						}
						log.Printf("[%v] SEND SUB %v", label, msg)
					}

				case 0x02:
					if i := channelContains(comm, routing[msg[1]]); i != -1 {
						routing[msg[1]] = append(routing[msg[1]][:i], routing[msg[1]][i+1:]...)
						for _, ch := range routers {
							if ch != comm {
								ch <- msg
							}
						}
						log.Printf("[%v] SEND UNSUB %v", label, msg)
					}

				case 0x03:
					for _, ch := range subscriptions[msg[1]] {
						ch <- msg
					}
					for _, ch := range routing[msg[1]] {
						ch <- msg
					}
					log.Printf("[%v] ROUTING %v", label, msg)

				default:
					log.Printf("[%v] INVALID MESSAGE %v", label, msg)
				}
				buf = rest
			}
		}
	}
	close(comm)
	quit <- true
	conn.Close()
	log.Printf("[%v] CLOSED", label)
}

func subscriber(conn net.Conn) {
	key := conn.RemoteAddr()
	label := fmt.Sprintf("%v / SUBSCRIBER", key)
	buf := make([]byte, 0, 32)
	comm := make(chan []byte)
	quit := make(chan bool)
	defer conn.Close()

	// WRITE
	go writer(conn, comm, label, quit)

	// READ
	for {
		read, err := bufferize(conn, 8)
		if err != nil {
			log.Printf("[%v] ERROR %v", label, err)
			return
		}
		buf = append(buf, read...)
		for {
			size, msg, rest := frame(buf)
			if size == 0 {
				break
			} else {
				log.Printf("[%v] HAVE GOT MESSAGE %v", label, msg)
				switch msg[0] {
				case 0x01:
					if channelContains(comm, subscriptions[msg[1]]) == -1 {
						subscriptions[msg[1]] = append(subscriptions[msg[1]], comm)
						for _, ch := range routers {
							ch <- msg
							log.Printf("something!")
						}
						log.Printf("[%v] SUBBING %v %v", label, msg, subscriptions)
					}

				case 0x02:
					if i := channelContains(comm, subscriptions[msg[1]]); i != -1 {
						subscriptions[msg[1]] = append(subscriptions[msg[1]][:i], subscriptions[msg[1]][i+1:]...)
						for _, ch := range routers {
							ch <- msg
						}
						log.Printf("[%v] UNSUBBING %v %v", label, msg, subscriptions)
					}

				default:
					log.Printf("[%v] INVALID MESSAGE %v", label, msg)
				}
				buf = rest
			}
		}
	}
	close(comm)
	quit <- true
	conn.Close()
	log.Printf("[%v] CLOSED", label)
}

func process(conn net.Conn) {
	defer conn.Close()
	label := fmt.Sprintf("%v / UNIDENTIFIED", conn.RemoteAddr())
	log.Printf("[router] PROCESS %v", label)

	conn.Write([]byte{0x02, 0x00, 0x00})

	first := make([]byte, 3)

	_, err := conn.Read(first)
	if err != nil {
		if err != io.EOF {
			log.Fatalf("[%s] ERROR: %v", label, err)
		}
		return
	}

	if first[0] != 0x02 {
		log.Fatalf("[%s] ERROR: WRONG MESSAGE", label)
	} else if first[1] != 0x00 {
		log.Fatalf("[%s] ERROR: WRONG MESSAGE", label)
	} else {
		switch first[2] {
		case 0x00:
			log.Printf("[%v] IDENTIFIED AS ROUTER", label)
			router(conn)
		case 0x01:
			log.Printf("[%v] IDENTIFIED AS SUBSCRIBER", label)
			subscriber(conn)
		case 0x02:
			log.Printf("[%v] IDENTIFIED AS PUBLISHER", label)
			publisher(conn)

		default:
			log.Printf("[%v] FAILED IDENTIFICATION", label)
		}
	}
}

func main() {
	if len(os.Args) == 1 {
		log.Fatalf("[router] NO PORT TO LISTEN TO, EXITING")
	} else {
		routers = make([]chan<- []byte, 0, 8)
		routing = map[byte][]chan<- []byte{
			0: {},
			1: {},
			2: {},
			4: {},
			5: {},
		}
		subscriptions = map[byte][]chan<- []byte{
			0: {},
			1: {},
			2: {},
			4: {},
			5: {},
		}

		myPort := os.Args[1]
		for _, port := range os.Args[2:] {
			conn, err := net.Dial("tcp", fmt.Sprintf("127.0.0.1:%s", port))
			if err != nil {
				log.Fatalf("[router] ERROR: %v", err)
				continue
			}
			go process(conn)
		}
		l, err := net.Listen("tcp", fmt.Sprintf("127.0.0.1:%s", myPort))
		if err != nil {
			log.Fatalf("[router] ERROR: %v", err)
		}
		defer l.Close()
		log.Printf("[router] READY TO ACCEPT CONNECTIONS")
		for {
			conn, err := l.Accept()
			if err != nil {
				log.Fatalf("[router] ERROR: %v", err)
				continue
			}
			go process(conn)
		}
	}
}
